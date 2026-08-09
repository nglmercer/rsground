use std::sync::LazyLock;

use actix_web::HttpRequest;
use chrono::{Duration, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{auth, env, http};
use crate::http_errors::HttpErrors;
use crate::utils::ArcStr;

/// The JWT secret is intentionally configurable, but a local default keeps the
/// guest-only development server usable after a fresh checkout. Deployments
/// should always provide `JWT_SECRET` through their environment.
pub static JWT_SECRET: LazyLock<String> = LazyLock::new(|| {
    std::env::var(env::JWT_SECRET)
        .ok()
        .filter(|secret| !secret.trim().is_empty())
        .unwrap_or_else(|| {
            log::warn!(
                "{} is not set; using an ephemeral local-development secret. Set it before deployment.",
                env::JWT_SECRET
            );
            format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
        })
});
const JWT_EXP: TimeDelta = Duration::hours(auth::JWT_EXPIRATION_HOURS);

pub(crate) fn validate_deployment_secret() -> Result<(), String> {
    let Some(secret) = std::env::var(env::JWT_SECRET)
        .ok()
        .filter(|secret| !secret.trim().is_empty())
    else {
        return Err(format!("{} must be set for deployment", env::JWT_SECRET));
    };

    if secret.len() < auth::MIN_DEPLOYMENT_SECRET_BYTES {
        return Err(format!(
            "{} must contain at least {} bytes for deployment",
            env::JWT_SECRET,
            auth::MIN_DEPLOYMENT_SECRET_BYTES
        ));
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RgUserData {
    pub id: ArcStr,
    pub name: ArcStr,
    pub is_guest: bool,
    pub exp: i64,
}

impl RgUserData {
    pub fn new(id: ArcStr, name: ArcStr, is_guest: bool) -> Self {
        let exp = Utc::now() + JWT_EXP;

        Self {
            id,
            name,
            is_guest,
            exp: exp.timestamp(),
        }
    }

    pub fn encode(self) -> Result<String, jsonwebtoken::errors::Error> {
        encode(self)
    }
}

pub fn encode(data: RgUserData) -> Result<String, jsonwebtoken::errors::Error> {
    jsonwebtoken::encode::<RgUserData>(
        &jsonwebtoken::Header::default(),
        &data,
        &jsonwebtoken::EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .inspect_err(|err| log::error!("Error encoding jwt: {err}"))
}

pub fn decode(token: impl AsRef<str>) -> Option<RgUserData> {
    let token_data = jsonwebtoken::decode::<RgUserData>(
        token.as_ref(),
        &jsonwebtoken::DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .inspect_err(|err| {
        log::error!("Error al decodificar JWT: {err}");
    })
    .ok()?;

    if Utc::now().timestamp() >= token_data.claims.exp {
        return None;
    }

    Some(token_data.claims)
}

pub fn get_auth_token(req: &HttpRequest) -> Option<&str> {
    req.headers()
        .get(http::AUTHORIZATION_HEADER)
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.strip_prefix(http::BEARER_PREFIX))
}

pub fn get_user_info(req: &HttpRequest) -> Result<RgUserData, HttpErrors> {
    let token = get_auth_token(req).ok_or_else(|| HttpErrors::NoTokenProvided)?;
    decode(token).ok_or(HttpErrors::InvalidJWT)
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, get_auth_token, get_user_info, RgUserData};
    use crate::constants::http;
    use crate::http_errors::HttpErrors;
    use actix_web::test::TestRequest;
    use chrono::Utc;

    #[test]
    fn round_trips_user_data_and_bearer_headers() {
        let data = RgUserData::new("user-id".into(), "Ada".into(), true);
        let token = encode(data).expect("test JWT should encode");
        let decoded = decode(&token).expect("test JWT should decode");

        assert_eq!(decoded.id.as_ref(), "user-id");
        assert_eq!(decoded.name.as_ref(), "Ada");
        assert!(decoded.is_guest);

        let request = TestRequest::default()
            .insert_header((
                http::AUTHORIZATION_HEADER,
                format!("{}{}", http::BEARER_PREFIX, token),
            ))
            .to_http_request();
        assert_eq!(get_auth_token(&request), Some(token.as_str()));
        assert_eq!(get_user_info(&request).unwrap().id.as_ref(), "user-id");
    }

    #[test]
    fn rejects_missing_invalid_and_expired_tokens() {
        let missing = TestRequest::default().to_http_request();
        assert!(matches!(
            get_user_info(&missing),
            Err(HttpErrors::NoTokenProvided)
        ));

        let invalid = TestRequest::default()
            .insert_header((
                http::AUTHORIZATION_HEADER,
                format!("{}not-a-jwt", http::BEARER_PREFIX),
            ))
            .to_http_request();
        assert!(matches!(
            get_user_info(&invalid),
            Err(HttpErrors::InvalidJWT)
        ));

        let wrong_scheme = TestRequest::default()
            .insert_header((http::AUTHORIZATION_HEADER, "Basic not-a-jwt"))
            .to_http_request();
        assert!(get_auth_token(&wrong_scheme).is_none());

        let expired = RgUserData {
            id: "expired".into(),
            name: "Expired".into(),
            is_guest: true,
            exp: Utc::now().timestamp() - 1,
        };
        let token = encode(expired).expect("expired JWT should still encode");
        assert!(decode(token).is_none());
    }
}
