use reqwest::header::*;
use reqwest::{Client, Method, Request, Url};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
}

pub fn get_user(token: &str, http_client: &Client) -> Result<User, Box<dyn std::error::Error>> {
    let mut req = Request::new(Method::GET, Url::parse("https://lichess.org/api/account")?);
    let headers = req.headers_mut();
    headers.insert(ACCEPT, "application/json".parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse()?);
    let response: User = http_client.execute(req)?.json()?;
    Ok(response)
}

pub fn oauth_token_from_code(
    code: &str,
    http_client: &Client,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Result<OAuthToken, Box<dyn std::error::Error>> {
    let mut req = Request::new(Method::POST, Url::parse("https://oauth.lichess.org/oauth")?);
    let body = req.body_mut();
    *body = Some(
        format!(
            "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&client_secret={}",
            code, redirect_uri, client_id, client_secret
        )
        .into(),
    );
    let headers = req.headers_mut();
    headers.insert(ACCEPT, "application/json".parse()?);
    headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".parse()?);
    let response: OAuthToken = http_client.execute(req)?.json()?;
    Ok(response)
}
