#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

use std::collections::HashMap;

use rocket::State;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;

use crate::domain::Flux;

mod domain;


#[database("mydb")]
struct MyDB(diesel::SqliteConnection);


#[get("/")]
fn index(sql: MyDB) -> Json<Vec<Flux>> {
    let mut output = Vec::<Flux>::new();
    // output
    Json(output)
}

#[post("/add", data = "<flux>")]
fn new(flux: Json<Flux>, sql: MyDB) {
}


fn main() {
    rocket::ignite()
        .attach(MyDB::fairing())
        .mount("/", routes![index, new])
        .launch();
}


#[cfg(test)]
mod test {
    #[test]
    fn test() {}
}
