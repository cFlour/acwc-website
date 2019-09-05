#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use chrono::prelude::*;
use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use rand::Rng;
use rocket::http::{Cookies, Status};
use rocket::request::Form;
use rocket::response::Redirect;
use rocket::State;
use rocket_contrib::templates::Template;
use std::collections::HashMap;

mod config;
mod db;
mod lichess;
mod session;

use config::Config;
use db::{AcwcDbClient, DbPool, Registration};
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
fn home(
    maybe_session: Option<Session>,
    db_pool: State<DbPool>,
) -> Result<Template, Box<dyn std::error::Error>> {
    match registration_state() {
        0 => Ok(Template::render("home", &context(&maybe_session))),
        1 => {
            if let Some(session) = &maybe_session {
                let maybe_registration = db_pool.find_registration(&session.lichess_id)?;
                if let Some(registration) = maybe_registration {
                    let mut ctx = context(&maybe_session);
                    ctx.insert(
                        "registration_status",
                        match registration.status {
                            db::STATUS_PENDING => "pending",
                            db::STATUS_APPROVED => "approved",
                            db::STATUS_REJECTED => "rejected",
                            _ => unreachable!(),
                        },
                    );
                    ctx.insert("td_comment", &registration.td_comment);
                    Ok(Template::render("registered", &ctx))
                } else {
                    Ok(Template::render(
                        "registrationform",
                        &context(&maybe_session),
                    ))
                }
            } else {
                Ok(Template::render(
                    "home_registrationopen",
                    &context(&maybe_session),
                ))
            }
        }
        2 => Ok(Template::render(
            "home_registrationclosed",
            &context(&maybe_session),
        )),
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

#[derive(FromForm)]
struct RegisterInfo {
    #[form(field = "optional-comment")]
    comment: Option<String>,
}

#[post("/register", data = "<form>", rank = 1)]
fn register(
    form: Form<RegisterInfo>,
    session: Session,
    db_pool: State<DbPool>,
) -> Result<Redirect, Box<dyn std::error::Error>> {
    if registration_state() == 1 && db_pool.find_registration(&session.lichess_id)?.is_none() {
        db_pool.insert_registration(&Registration {
            lichess_id: session.lichess_id,
            lichess_username: session.lichess_username,
            status: db::STATUS_PENDING,
            registrant_comment: form.comment.clone().unwrap_or(String::from("")),
            td_comment: String::from(""),
            special: false,
        })?;
    }

    Ok(Redirect::to(uri!(home)))
}

#[post("/register", rank = 2)]
fn register_needs_authentication() -> Redirect {
    Redirect::to(uri!(home))
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
    let configuration = config::from_file("Config.toml").expect("failed to load config");

    let manager =
        PostgresConnectionManager::new((&configuration.postgres_options).parse().unwrap(), NoTls);
    let pool = Pool::new(manager).unwrap();

    rocket::ignite()
        .attach(Template::fairing())
        .manage(configuration)
        .manage(reqwest::Client::new())
        .manage(pool)
        .mount(
            "/",
            routes![
                home,
                rules_2019,
                auth,
                oauth_redirect,
                register,
                register_needs_authentication,
                logout
            ],
        )
        .launch();
}
