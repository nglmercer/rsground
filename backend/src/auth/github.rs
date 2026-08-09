use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;

const GITHUB_CLIENT_ID_ENV: &str = "GITHUB_CLIENT_ID";
const GITHUB_CLIENT_SECRET_ENV: &str = "GITHUB_CLIENT_SECRET";
const GITHUB_CALLBACK_ENV: &str = "GITHUB_CALLBACK";

const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";

const USER_AGENT: &str = "RustLangEs/rsground";

#[derive(Deserialize, Debug)]
pub struct GitHubUser {
    pub login: String,
    pub avatar_url: String,
}

pub fn get_oauth_client() -> Option<BasicClient> {
    let client_id = std::env::var(GITHUB_CLIENT_ID_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());

    let client_secret = std::env::var(GITHUB_CLIENT_SECRET_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());

    let callback = std::env::var(GITHUB_CALLBACK_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());

    let (Some(client_id), Some(client_secret), Some(callback)) =
        (client_id, client_secret, callback)
    else {
        log::warn!(
            "GitHub OAuth is not configured; guest login remains available. \
             Set {GITHUB_CLIENT_ID_ENV}, {GITHUB_CLIENT_SECRET_ENV}, and \
             {GITHUB_CALLBACK_ENV} to enable GitHub login."
        );
        return None;
    };

    let auth_url = AuthUrl::new(GITHUB_AUTH_URL.to_owned())
        .expect("GitHub authorization URL is a compile-time constant");

    let token_url = TokenUrl::new(GITHUB_TOKEN_URL.to_owned())
        .expect("GitHub token URL is a compile-time constant");

    let redirect_uri = match RedirectUrl::new(callback) {
        Ok(url) => url,
        Err(error) => {
            log::error!("Invalid {GITHUB_CALLBACK_ENV}: {error}");
            return None;
        }
    };

    Some(
        BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_uri),
    )
}

pub async fn fetch_user(access_token: &str) -> Result<GitHubUser, reqwest::Error> {
    let client = reqwest::Client::new();

    let res = client
        .get(GITHUB_USER_URL)
        .header("User-Agent", USER_AGENT)
        .bearer_auth(access_token)
        .send()
        .await?;

    let user = res.json::<GitHubUser>().await?;

    Ok(user)
}
