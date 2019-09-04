#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket_contrib::templates::Template;

#[get("/")]
fn home() -> Template {
    Template::render("home", &std::collections::HashMap::<u8, u8>::new())
}

#[get("/rules/2019")]
fn rules_2019() -> Template {
    Template::render("rules2019", &std::collections::HashMap::<u8, u8>::new())
}

fn main() {
    rocket::ignite()
        .attach(Template::fairing())
        .mount("/", routes![home, rules_2019])
        .launch();
}
