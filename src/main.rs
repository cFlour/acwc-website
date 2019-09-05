#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use chrono::prelude::*;
use rand::Rng;
use rocket::http::{Cookies, Status};
use rocket::request::Form;
use rocket::response::Redirect;
use rocket::State;
use rocket_contrib::templates::Template;
use std::collections::HashMap;
use std::sync::RwLock;

mod config;
mod lichess;
mod session;

use config::Config;
use session::Session;

fn context<'a>(maybe_session: &'a Option<Session>) -> HashMap<&'static str, &'a str> {
    let mut ctx: HashMap<&'static str, &'a str> = HashMap::new();
    if let Some(session) = maybe_session {
        ctx.insert("username", &session.lichess_username);
    }
    ctx
}

fn registration_state() -> i32 {
    let register_start = Utc.ymd(2019, 9, 4).and_hms(20, 0, 0);
    let register_end = Utc.ymd(2019, 9, 27).and_hms(20, 0, 0);
    let now = Utc::now();
    if now >= register_end {
        2 // registration ended
    } else if now >= register_start {
        1 // registration open
    } else {
        0 // registration not yet open
    }
}

#[get("/")]
fn home(session: Option<Session>) -> Template {
    match registration_state() {
        0 => Template::render("home", &context(&session)),
        1 => Template::render("home_registrationopen", &context(&session)),
        2 => Template::render("home_registrationclosed", &context(&session)),
        _ => unreachable!(),
    }
}

#[get("/rules/2019")]
fn rules_2019(session: Option<Session>) -> Template {
    Template::render("rules2019", &context(&session))
}

#[get("/oauth_redirect?<code>&<state>")]
fn oauth_redirect(
    mut cookies: Cookies<'_>,
    code: String,
    state: String,
    config: State<Config>,
    http_client: State<reqwest::Client>,
) -> Result<Result<Template, Status>, Box<dyn std::error::Error>> {
    match session::pop_oauth_state(&mut cookies).map(|v| v == state) {
        Some(true) => {
            let token = lichess::oauth_token_from_code(
                &code,
                &http_client,
                &config.oauth_client_id,
                &config.oauth_client_secret,
                &format!("{}/oauth_redirect", config.server_url),
            )
            .unwrap();
            let user = lichess::get_user(&token.access_token, &http_client).unwrap();
            session::set_session(
                cookies,
                Session {
                    lichess_id: user.id,
                    lichess_username: user.username,
                },
            )?;
            Ok(Ok(Template::render("redirect", &context(&None))))
        }
        _ => Ok(Err(Status::BadRequest)),
    }
}

fn random_oauth_state() -> Result<String, std::str::Utf8Error> {
    let mut rng = rand::thread_rng();
    let mut oauth_state_bytes: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    for x in &mut oauth_state_bytes {
        *x = (rng.gen::<u8>() % 26) + 97;
    }
    Ok(std::str::from_utf8(&oauth_state_bytes)?.to_string())
}

#[get("/auth")]
fn auth(
    config: State<Config>,
    cookies: Cookies<'_>,
) -> Result<Redirect, Box<dyn std::error::Error>> {
    let oauth_state = random_oauth_state()?;
    session::set_oauth_state_cookie(cookies, &oauth_state);

    let url = format!("https://oauth.lichess.org/oauth/authorize?response_type=code&client_id={}&redirect_uri={}/oauth_redirect&scope=&state={}",
    config.oauth_client_id, config.server_url, oauth_state);

    Ok(Redirect::to(url))
}

#[post("/logout")]
fn logout(cookies: Cookies<'_>) -> Template {
    session::remove_session(cookies);
    Template::render("redirect", &context(&None))
}

fn main() {
    rocket::ignite()
        .attach(Template::fairing())
        .manage(config::from_file("Config.toml").expect("failed to load config"))
        .manage(reqwest::Client::new())
        .mount("/", routes![home, rules_2019, auth, oauth_redirect, logout])
        .launch();
}
