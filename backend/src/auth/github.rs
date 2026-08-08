use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GitHubUser {
    pub login: String,
    pub avatar_url: String,
}

pub fn get_oauth_client() -> Option<BasicClient> {
    let client_id = std::env::var("GITHUB_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let callback = std::env::var("GITHUB_CALLBACK")
        .ok()
        .filter(|value| !value.trim().is_empty());

    let (Some(client_id), Some(client_secret), Some(callback)) =
        (client_id, client_secret, callback)
    else {
        log::warn!(
            "GitHub OAuth is not configured; guest login remains available. Set GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, and GITHUB_CALLBACK to enable GitHub login."
        );
        return None;
    };

    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .expect("GitHub authorization URL is a compile-time constant");
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .expect("GitHub token URL is a compile-time constant");

    let redirect_uri = match RedirectUrl::new(callback) {
        Ok(url) => url,
        Err(error) => {
            log::error!("Invalid GITHUB_CALLBACK: {error}");
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
    let user_url = "https://api.github.com/user";

    let res = client
        .get(user_url)
        .header("User-Agent", "RustLangEs/rsground")
        .bearer_auth(access_token)
        .send()
        .await?;

    let user: GitHubUser = res.json().await?;
    Ok(user)
}
