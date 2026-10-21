//! Outbound-target validation for URLs supplied by users.
//!
//! The registry fetches two classes of URL that a user controls: webhook
//! delivery targets (`webhook_delivery.rs`) and anything else a handler is
//! given to call. Without a host check those are a server-side request
//! forgery primitive — the caller picks an address and the server reaches it
//! from inside the network, returning or logging the response.
//!
//! [`validate_outbound_target`] resolves the host and rejects any target that
//! lands on a loopback, private, link-local, unspecified or multicast address.
//!
//! **Call it at delivery as well as at registration.** A name that resolved to
//! a public address when the webhook was created can resolve to `127.0.0.1`
//! later; checking only at registration leaves the rebinding window open.

use std::net::IpAddr;

/// Addresses an outbound request must never reach.
///
/// IPv4 link-local (169.254.0.0/16) covers the cloud metadata endpoint at
/// 169.254.169.254, which is the usual target. The IPv6 ranges are matched by
/// hand because `is_unique_local` and `is_unicast_link_local` are still
/// unstable.
pub fn is_blocked_address(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_multicast()
                // 100.64.0.0/10, carrier-grade NAT
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // fc00::/7, unique local
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // fe80::/10, link-local unicast
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // An IPv4-mapped address is only as safe as the address it maps.
                || v6.to_ipv4_mapped().is_some_and(|m| is_blocked_address(&IpAddr::V4(m)))
        }
    }
}

/// True when the deployment has opted into reaching private addresses.
///
/// Off unless `WEBHOOK_ALLOW_PRIVATE_TARGETS=true`, so a production deployment
/// that sets nothing is protected. Local development and the integration tests
/// set it to keep `http://localhost` targets working.
pub fn private_targets_allowed() -> bool {
    std::env::var("WEBHOOK_ALLOW_PRIVATE_TARGETS")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

/// Reject a user-supplied URL that would make the server call itself or its
/// neighbours.
///
/// Returns `Ok(())` when the scheme is http/https and every address the host
/// resolves to is routable. A host that does not resolve is rejected: an
/// unresolvable target cannot be delivered to anyway, and accepting it would
/// mean the check is skipped whenever DNS is briefly unavailable.
pub async fn validate_outbound_target(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url.trim()).map_err(|e| format!("Invalid URL: {}", e))?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("URL scheme '{}' is not allowed", other)),
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;

    if private_targets_allowed() {
        return Ok(());
    }

    // A literal address needs no lookup, and resolving one would be a no-op.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return if is_blocked_address(&ip) {
            Err(format!("URL resolves to a non-routable address ({})", ip))
        } else {
            Ok(())
        };
    }

    let port = parsed.port_or_known_default().unwrap_or(443);
    let resolved = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("Could not resolve host '{}': {}", host, e))?;

    let mut any = false;
    for addr in resolved {
        any = true;
        if is_blocked_address(&addr.ip()) {
            return Err(format!(
                "URL resolves to a non-routable address ({})",
                addr.ip()
            ));
        }
    }

    if !any {
        return Err(format!("Host '{}' did not resolve to any address", host));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn blocks_loopback_and_private_v4() {
        for addr in [
            "127.0.0.1",
            "10.0.0.5",
            "172.16.0.1",
            "192.168.1.1",
            "0.0.0.0",
            "255.255.255.255",
            "100.64.0.1",
        ] {
            assert!(is_blocked_address(&ip(addr)), "{} should be blocked", addr);
        }
    }

    #[test]
    fn blocks_the_metadata_endpoint() {
        assert!(is_blocked_address(&ip("169.254.169.254")));
    }

    #[test]
    fn blocks_loopback_and_local_v6() {
        for addr in [
            "::1",
            "::",
            "fc00::1",
            "fd00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(is_blocked_address(&ip(addr)), "{} should be blocked", addr);
        }
    }

    #[test]
    fn allows_routable_addresses() {
        for addr in ["8.8.8.8", "1.1.1.1", "93.184.216.34", "2606:4700::1111"] {
            assert!(!is_blocked_address(&ip(addr)), "{} should be allowed", addr);
        }
    }

    #[tokio::test]
    async fn rejects_literal_loopback_url() {
        let err = validate_outbound_target("http://127.0.0.1:5432/x")
            .await
            .unwrap_err();
        assert!(err.contains("non-routable"), "unexpected error: {}", err);
    }

    #[tokio::test]
    async fn rejects_literal_metadata_url() {
        assert!(
            validate_outbound_target("https://169.254.169.254/latest/meta-data/")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_non_http_schemes() {
        for url in [
            "file:///etc/passwd",
            "gopher://example.com/",
            "ftp://example.com/",
        ] {
            assert!(
                validate_outbound_target(url).await.is_err(),
                "{} should be rejected",
                url
            );
        }
    }

    #[tokio::test]
    async fn allows_a_routable_literal() {
        assert!(validate_outbound_target("https://8.8.8.8/healthz")
            .await
            .is_ok());
    }
}
