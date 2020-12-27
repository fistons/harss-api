#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

use std::collections::HashMap;

use rocket::State;
use rocket_contrib::json::Json;

use crate::domain::*;

use self::diesel::prelude::*;
use self::schema::flux::dsl::*;

pub mod schema;
pub mod domain;

#[database("mydb")]
struct MyDB(diesel::SqliteConnection);



#[get("/")]
fn index(sql: MyDB) -> &'static str {
    diesel::select(flux).get_result(&sql);
    "coucou"
}

fn main() {
    rocket::ignite()
        .attach(MyDB::fairing())
        .mount("/", routes![index])
        .launch();
}
