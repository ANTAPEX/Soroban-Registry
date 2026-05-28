use crate::{error::ApiError, state::AppState};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{
    extract::Request, http::header, http::StatusCode, middleware::Next, response::Response,
};
use chrono::{Duration, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use stellar_strkey::{Strkey, ed25519::PublicKey as StellarPublicKey};

pub const MIN_JWT_SECRET_LEN: usize = 32;

/// Authenticated user extracted from a valid Bearer JWT.
/// The `sub` claim is expected to be the publisher's Stellar address,
/// and `publisher_id` is derived by looking up the publisher in the DB.
/// For simplicity (matching the existing subscription_handlers pattern),
/// we store the sub as a string and expose a UUID parsed from it when possible,
/// falling back to a nil UUID so callers can handle the error themselves.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The `sub` claim from the JWT (Stellar address / publisher identifier)
    pub stellar_address: String,
    /// Publisher UUID — parsed from sub if it is a UUID, otherwise nil
    pub publisher_id: uuid::Uuid,
    /// Database user id (publisher primary key)
    pub id: uuid::Uuid,
    /// Full claims for callers that need them
    pub claims: AuthClaims,
}

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let auth_manager =
            AuthManager::from_env().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let claims = auth_manager
            .validate_jwt(auth_header)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let publisher_id = uuid::Uuid::parse_str(&claims.sub).unwrap_or(uuid::Uuid::nil());

        Ok(AuthenticatedUser {
            stellar_address: claims.sub.clone(),
            publisher_id,
            id: publisher_id,
            claims,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String,
    pub publisher_id: uuid::Uuid,
    pub iat: i64,
    pub exp: i64,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub admin: bool,
}

#[derive(Debug, Clone)]
pub struct ChallengeRecord {
    pub nonce: String,
    pub expires_at: i64,
}

pub struct AuthManager {
    challenges: HashMap<String, ChallengeRecord>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthConfigError {
    MissingJwtSecret,
    JwtSecretTooShort { min_len: usize, actual_len: usize },
}

impl fmt::Display for AuthConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthConfigError::MissingJwtSecret => write!(f, "JWT_SECRET must be set"),
            AuthConfigError::JwtSecretTooShort {
                min_len,
                actual_len,
            } => write!(
                f,
                "JWT_SECRET must be at least {} characters (got {})",
                min_len, actual_len
            ),
        }
    }
}

impl std::error::Error for AuthConfigError {}

impl AuthManager {
    pub fn new(secret: String) -> Self {
        Self {
            challenges: HashMap::new(),
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn from_env() -> Result<Self, AuthConfigError> {
        let secret = std::env::var("JWT_SECRET").map_err(|_| AuthConfigError::MissingJwtSecret)?;
        Self::validate_jwt_secret(&secret)?;
        Ok(Self::new(secret))
    }

    fn validate_jwt_secret(secret: &str) -> Result<(), AuthConfigError> {
        let actual_len = secret.len();
        if actual_len < MIN_JWT_SECRET_LEN {
            return Err(AuthConfigError::JwtSecretTooShort {
                min_len: MIN_JWT_SECRET_LEN,
                actual_len,
            });
        }
        Ok(())
    }

    pub fn create_challenge(&mut self, address: &str) -> String {
        let nonce: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let expires_at = (Utc::now() + Duration::minutes(5)).timestamp();
        self.challenges.insert(
            address.to_string(),
            ChallengeRecord {
                nonce: nonce.clone(),
                expires_at,
            },
        );
        nonce
    }

    pub fn verify_and_issue_jwt(
        &mut self,
        address: &str,
        public_key_hex: &str,
        signature_hex: &str,
        publisher_id: uuid::Uuid,
        scopes: Vec<String>,
        expires_in_seconds: u64,
    ) -> Result<String, &'static str> {
        let challenge = self
            .challenges
            .remove(address)
            .ok_or("challenge_not_found")?;
        if Utc::now().timestamp() > challenge.expires_at {
            return Err("challenge_expired");
        }
        let public_key = decode_hex_32(public_key_hex).ok_or("invalid_public_key_hex")?;
        let address_public_key = decode_stellar_public_key(address).ok_or("invalid_address")?;
        if address_public_key != public_key {
            return Err("address_public_key_mismatch");
        }
        let signature = decode_hex_64(signature_hex).ok_or("invalid_signature_hex")?;
        let vk = VerifyingKey::from_bytes(&public_key).map_err(|_| "invalid_public_key")?;
        let sig = Signature::from_bytes(&signature);
        vk.verify(challenge.nonce.as_bytes(), &sig)
            .map_err(|_| "invalid_signature")?;
        let iat = Utc::now().timestamp();
        let expires_in_seconds = expires_in_seconds.clamp(300, 30 * 24 * 60 * 60);
        let exp = (Utc::now() + Duration::seconds(expires_in_seconds as i64)).timestamp();
        let claims = AuthClaims {
            sub: address.to_string(),
            publisher_id,
            iat,
            exp,
            scopes,
            role: None,
            admin: false,
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(|_| "jwt_encode_failed")
    }

    pub fn validate_jwt(&self, token: &str) -> Result<AuthClaims, &'static str> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        decode::<AuthClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| "invalid_token")
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &AppState) -> Result<Self, ApiError> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("Missing authorization header"))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::unauthorized(
                "Invalid authorization header format",
            ));
        }

        let token = &auth_header[7..];
        let auth_manager = AuthManager::from_env()
            .map_err(|_| ApiError::internal("Authentication configuration error"))?;

        let claims = auth_manager
            .validate_jwt(token)
            .map_err(|_| ApiError::unauthorized("Invalid or expired token"))?;

        Ok(claims)
    }
}

fn extract_bearer_token(req: &Request) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn is_admin(claims: &AuthClaims) -> bool {
    claims.admin || matches!(claims.role.as_deref(), Some("admin" | "ADMIN" | "Admin"))
}

pub async fn require_admin(req: Request, next: Next) -> Result<Response, ApiError> {
    let Some(token) = extract_bearer_token(&req) else {
        return Err(ApiError::unauthorized(
            "Authorization header with Bearer token is required",
        ));
    };

    let auth = AuthManager::from_env()
        .map_err(|_| ApiError::internal("Authentication configuration error"))?;
    let claims = auth
        .validate_jwt(token)
        .map_err(|_| ApiError::unauthorized("Invalid or expired authentication token"))?;

    if !is_admin(&claims) {
        return Err(ApiError::forbidden(
            "Administrative privileges are required for this endpoint",
        ));
    }

    Ok(next.run(req).await)
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    let bytes = decode_hex(value)?;
    let mut out = [0u8; 32];
    if bytes.len() != out.len() {
        return None;
    }
    out.copy_from_slice(&bytes);
    Some(out)
}

fn decode_hex_64(value: &str) -> Option<[u8; 64]> {
    let bytes = decode_hex(value)?;
    let mut out = [0u8; 64];
    if bytes.len() != out.len() {
        return None;
    }
    out.copy_from_slice(&bytes);
    Some(out)
}

fn decode_stellar_public_key(value: &str) -> Option<[u8; 32]> {
    match Strkey::from_string(value).ok()? {
        Strkey::PublicKeyEd25519(StellarPublicKey(bytes)) => Some(bytes),
        _ => None,
    }
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

    fn hex_encode(data: &[u8]) -> String {
        data.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    #[test]
    fn challenge_verify_and_jwt_works() {
        let mut auth = AuthManager::new("test-secret".to_string());
        let seed = [7u8; 32];
        let sk = SigningKey::from_bytes(&seed);
        let address = StellarPublicKey(*sk.verifying_key().as_bytes()).to_string();
        let vk_hex = hex_encode(sk.verifying_key().as_bytes());
        let nonce = auth.create_challenge(&address);
        let sig = sk.sign(nonce.as_bytes());
        let token = auth
            .verify_and_issue_jwt(
                &address,
                &vk_hex,
                &hex_encode(&sig.to_bytes()),
                uuid::Uuid::nil(),
                vec!["read".to_string()],
                3_600,
            )
            .expect("jwt must be issued");
        let claims = auth.validate_jwt(&token).expect("token must be valid");
        assert_eq!(claims.sub, address);
        assert_eq!(claims.scopes, vec!["read".to_string()]);
    }

    #[test]
    fn nonce_is_single_use() {
        let mut auth = AuthManager::new("test-secret".to_string());
        let seed = [9u8; 32];
        let sk = SigningKey::from_bytes(&seed);
        let address = StellarPublicKey(*sk.verifying_key().as_bytes()).to_string();
        let vk_hex = hex_encode(sk.verifying_key().as_bytes());
        let nonce = auth.create_challenge(&address);
        let sig = sk.sign(nonce.as_bytes());
        let sig_hex = hex_encode(&sig.to_bytes());
        let first = auth.verify_and_issue_jwt(
            &address,
            &vk_hex,
            &sig_hex,
            uuid::Uuid::nil(),
            vec![],
            3_600,
        );
        assert!(first.is_ok());
        let second = auth.verify_and_issue_jwt(
            &address,
            &vk_hex,
            &sig_hex,
            uuid::Uuid::nil(),
            vec![],
            3_600,
        );
        assert!(second.is_err());
    }

    #[test]
    fn jwt_secret_length_is_enforced() {
        let too_short = "a".repeat(MIN_JWT_SECRET_LEN - 1);
        let result = AuthManager::validate_jwt_secret(&too_short);
        assert!(matches!(
            result,
            Err(AuthConfigError::JwtSecretTooShort {
                min_len: MIN_JWT_SECRET_LEN,
                actual_len: _
            })
        ));

        let valid = "a".repeat(MIN_JWT_SECRET_LEN);
        assert!(AuthManager::validate_jwt_secret(&valid).is_ok());
    }
}
