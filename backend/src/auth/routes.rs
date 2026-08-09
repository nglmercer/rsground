use actix_error_proc::{proof_route, HttpResult};
use actix_web::{
    cookie::{time::Duration as CookieDuration, Cookie, SameSite},
    get, web, HttpRequest, HttpResponse, Responder,
};
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::jwt::RgUserData;
use crate::auth::{github, jwt};
use crate::constants::{auth as auth_constants, env, environment, http, json, limits};
use crate::http_errors::HttpErrors;
use crate::state::AppState;
use crate::utils::ArcStr;

pub struct OAuthData {
    pub client: Option<github::GithubOAuthClient>,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub code: String,
    pub state: String,
}

fn oauth_cookie_secure() -> bool {
    std::env::var(env::ENVIRONMENT).is_ok_and(|value| {
        value.eq_ignore_ascii_case(environment::PRODUCTION)
            || value.eq_ignore_ascii_case(environment::PRODUCTION_ALIAS)
    })
}

fn oauth_state_cookie(value: impl Into<String>) -> Cookie<'static> {
    Cookie::build(auth_constants::OAUTH_STATE_COOKIE, value.into())
        .path(auth_constants::OAUTH_COOKIE_PATH)
        .http_only(true)
        .same_site(if oauth_cookie_secure() {
            SameSite::None
        } else {
            SameSite::Lax
        })
        .secure(oauth_cookie_secure())
        .max_age(CookieDuration::minutes(
            auth_constants::OAUTH_COOKIE_MAX_AGE_MINUTES,
        ))
        .finish()
}

fn clear_oauth_state_cookie() -> Cookie<'static> {
    Cookie::build(auth_constants::OAUTH_STATE_COOKIE, "")
        .path(auth_constants::OAUTH_COOKIE_PATH)
        .http_only(true)
        .same_site(if oauth_cookie_secure() {
            SameSite::None
        } else {
            SameSite::Lax
        })
        .secure(oauth_cookie_secure())
        .max_age(CookieDuration::ZERO)
        .finish()
}

#[get("/auth")]
pub async fn auth(oauth: web::Data<OAuthData>) -> impl Responder {
    let Some(client) = oauth.client.as_ref() else {
        return HttpResponse::ServiceUnavailable().json(serde_json::json!({
            (json::ERROR): HttpErrors::OAuthNotConfigured.to_string()
        }));
    };

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(auth_constants::OAUTH_SCOPE_READ_USER.to_owned()))
        .url();

    HttpResponse::Found()
        .cookie(oauth_state_cookie(csrf_token.secret().to_owned()))
        .append_header((http::LOCATION_HEADER, auth_url.to_string()))
        .finish()
}

#[get("/auth/me")]
pub async fn me(req: HttpRequest) -> HttpResult<HttpErrors> {
    let user_info = jwt::get_user_info(&req)?;

    Ok(HttpResponse::Ok().json(user_info))
}

#[get("/auth/callback")]
async fn callback(
    state: web::Data<AppState>,
    query: web::Query<AuthRequest>,
    oauth_data: web::Data<OAuthData>,
    req: HttpRequest,
) -> HttpResult<HttpErrors> {
    let Some(client) = oauth_data.client.as_ref() else {
        return Err(HttpErrors::OAuthNotConfigured);
    };

    let Some(state_cookie) = req.cookie(auth_constants::OAUTH_STATE_COOKIE) else {
        return Err(HttpErrors::InvalidOAuthState);
    };
    if state_cookie.value() != query.state {
        return Err(HttpErrors::InvalidOAuthState);
    }

    let code = AuthorizationCode::new(query.code.clone());
    let http_client = reqwest::Client::new();

    let token = client
        .exchange_code(code)
        .request_async(&http_client)
        .await
        .map_err(|err| err.to_string())
        .map_err(HttpErrors::CodeExchange)
        .inspect_err(|err| log::error!("{err}"))?;

    let access_token = token.access_token().secret();
    let github_user = github::fetch_user(access_token)
        .await
        .map_err(HttpErrors::GithubUserFetch)
        .inspect_err(|err| log::error!("{err}"))?;

    let user_data = RgUserData::new(
        github_user.login.as_str().into(),
        github_user.login.as_str().into(),
        false,
    );

    if !state
        .add_username(user_data.id.clone(), user_data.name.clone())
        .await
    {
        return Err(HttpErrors::UserLimitReached);
    }

    let jwt = user_data.encode()?;

    let response = HttpResponse::Ok()
        .cookie(clear_oauth_state_cookie())
        .json(serde_json::json!({
            (json::JWT): jwt,
            (json::ID): github_user.login,
            (json::NAME): github_user.login,
            (json::AVATAR_URL): github_user.avatar_url,
            (json::IS_GUEST): false,
        }));

    Ok(response)
}

#[derive(Deserialize)]
struct GuestLoginRequest {
    guest_name: String,
}

#[proof_route(post("/auth/guest"))]
async fn login_guest(
    state: web::Data<AppState>,
    body: web::Json<GuestLoginRequest>,
) -> HttpResult<HttpErrors> {
    let guest_name = body.guest_name.trim();
    if guest_name.is_empty() || guest_name.chars().count() > limits::MAX_GUEST_NAME_CHARS {
        return Err(HttpErrors::InvalidGuestName);
    }

    let guest_uuid = Uuid::new_v4().to_string();

    let user_data = RgUserData::new(guest_uuid.as_str().into(), guest_name.into(), true);

    if !state
        .add_username(user_data.id.clone(), user_data.name.clone())
        .await
    {
        return Err(HttpErrors::UserLimitReached);
    }

    let jwt = user_data.encode()?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        (json::JWT): jwt,
        (json::ID): guest_uuid,
        (json::NAME): guest_name,
        (json::IS_GUEST): true,
    })))
}
#[derive(Deserialize)]
struct UpdateNameRequest {
    new_name: ArcStr,
}

#[proof_route(post("/auth/update"))]
async fn update_name(
    state: web::Data<AppState>,
    body: web::Json<UpdateNameRequest>,
    req: actix_web::HttpRequest,
) -> HttpResult<HttpErrors> {
    let token_data = jwt::get_user_info(&req)?;

    if !token_data.is_guest {
        return Err(HttpErrors::GithubNameChange);
    }

    let uuid = token_data.id;
    let new_name = body.into_inner().new_name;
    let new_name = new_name.trim();
    if new_name.is_empty() || new_name.chars().count() > limits::MAX_GUEST_NAME_CHARS {
        return Err(HttpErrors::InvalidGuestName);
    }
    let new_name: ArcStr = new_name.into();

    let user_data = RgUserData::new(uuid.clone(), new_name.clone(), true);

    if !state
        .add_username(user_data.id.clone(), user_data.name.clone())
        .await
    {
        return Err(HttpErrors::UserLimitReached);
    }

    let jwt = user_data.encode()?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        (json::JWT): jwt,
        (json::ID): uuid,
        (json::NAME): new_name,
        (json::IS_GUEST): true,
    })))
}
