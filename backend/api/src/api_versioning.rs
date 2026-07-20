use axum::{
    body::Body,
    http::{
        header::{HeaderName, HeaderValue},
        Request,
    },
    middleware::Next,
    response::Response,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use utoipa::ToSchema;

#[derive(Clone, Copy)]
pub enum ApiVersion {
    V1,
    V2,
}

impl ApiVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "v1",
            ApiVersion::V2 => "v2",
        }
    }
}

pub struct ApiVersionMetrics {
    pub v1_calls: AtomicU64,
    pub v2_calls: AtomicU64,
    pub deprecated_alias_calls: AtomicU64,
}

impl ApiVersionMetrics {
    pub fn new() -> Self {
        Self {
            v1_calls: AtomicU64::new(0),
            v2_calls: AtomicU64::new(0),
            deprecated_alias_calls: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> ApiVersionMetricsSnapshot {
        ApiVersionMetricsSnapshot {
            v1_calls: self.v1_calls.load(Ordering::Relaxed),
            v2_calls: self.v2_calls.load(Ordering::Relaxed),
            deprecated_alias_calls: self.deprecated_alias_calls.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct ApiVersionMetricsSnapshot {
    pub v1_calls: u64,
    pub v2_calls: u64,
    pub deprecated_alias_calls: u64,
}

#[derive(Clone)]
pub struct ApiVersionMiddlewareState {
    pub metrics: Arc<ApiVersionMetrics>,
    pub version: ApiVersion,
    pub deprecated_alias: bool,
    pub sunset: Option<&'static str>,
}

const HEADER_API_VERSION: HeaderName = HeaderName::from_static("x-api-version");

pub async fn version_middleware(
    axum::extract::State(state): axum::extract::State<ApiVersionMiddlewareState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    match state.version {
        ApiVersion::V1 => {
            state.metrics.v1_calls.fetch_add(1, Ordering::Relaxed);
        }
        ApiVersion::V2 => {
            state.metrics.v2_calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    if state.deprecated_alias {
        state
            .metrics
            .deprecated_alias_calls
            .fetch_add(1, Ordering::Relaxed);
    }

    let mut res = next.run(req).await;

    res.headers_mut().insert(
        HEADER_API_VERSION,
        HeaderValue::from_static(match state.version {
            ApiVersion::V1 => "v1",
            ApiVersion::V2 => "v2",
        }),
    );

    if state.deprecated_alias {
        res.headers_mut()
            .insert(HeaderName::from_static("deprecation"), HeaderValue::from_static("true"));
        if let Some(sunset) = state.sunset {
            if let Ok(value) = HeaderValue::from_str(sunset) {
                res.headers_mut()
                    .insert(HeaderName::from_static("sunset"), value);
            }
        }
        res.headers_mut().insert(
            HeaderName::from_static("warning"),
            HeaderValue::from_static("299 - \"Deprecated API path. Use /api/v1 or /api/v2.\""),
        );
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tower::ServiceExt;

    #[tokio::test]
    async fn adds_version_header_and_deprecation_headers() {
        let metrics = Arc::new(ApiVersionMetrics::new());
        let state = ApiVersionMiddlewareState {
            metrics: Arc::clone(&metrics),
            version: ApiVersion::V1,
            deprecated_alias: true,
            sunset: Some("Wed, 31 Dec 2026 00:00:00 GMT"),
        };

        let app = Router::new()
            .route("/x", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(state, version_middleware));

        let res = app
            .oneshot(Request::builder().uri("/x").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            res.headers()
                .get("x-api-version")
                .unwrap()
                .to_str()
                .unwrap(),
            "v1"
        );
        assert_eq!(
            res.headers()
                .get("deprecation")
                .unwrap()
                .to_str()
                .unwrap(),
            "true"
        );
        assert!(res.headers().get("sunset").is_some());
        assert_eq!(metrics.snapshot().deprecated_alias_calls, 1);
    }
}
