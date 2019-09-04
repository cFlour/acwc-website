#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use chrono::prelude::*;
use rocket_contrib::templates::Template;

fn empty_context() -> std::collections::HashMap::<u8, u8> {
    std::collections::HashMap::<u8, u8>::new()
}

fn registration_state() -> i32 {
    let register_start = Utc.ymd(2019, 9, 5).and_hms(20, 0, 0);
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
fn home() -> Template {
    match registration_state() {
        0 => Template::render("home", &empty_context()),
        1 => Template::render("home_registrationopen", &empty_context()),
        2 => Template::render("home_registrationclosed", &empty_context()),
        _ => unreachable!(),
    }

}

#[get("/rules/2019")]
fn rules_2019() -> Template {
    Template::render("rules2019", &empty_context())
}

fn main() {
    rocket::ignite()
        .attach(Template::fairing())
        .mount("/", routes![home, rules_2019])
        .launch();
}
