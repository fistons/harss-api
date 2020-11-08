#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use rocket::State;
use rocket_contrib::json::Json;
use rustbreak::deser::Yaml;
use rustbreak::FileDatabase;

use crate::domain::Flux;
use std::fs::File;
use std::ops::Add;

mod domain;

type Loic = FileDatabase::<HashMap<u32, Flux>, Yaml>;

#[get("/")]
fn index(db: State<Loic>) -> Json<Vec<Flux>> {
    let mut output = Vec::<&Flux>::new();
    db.read(|db| {
       for (key, flux) in db.iter() {
           // output.push(flux.)
       }
    });
    // output
    Json(vec![])
}

#[post("/add", data = "<flux>")]
fn new(flux: Json<Flux>, db: State<Loic>) {
    db.write(|db| {
        let flux = flux.0;
        db.insert(flux.id, flux);
    });
    db.save();
}


fn main() {

    let db : Loic = FileDatabase::load_from_path_or("/home/eric/pouet.yaml", HashMap::<u32, Flux>::new()).unwrap();
    let _ = db.load();

    rocket::ignite()
        .manage(db)
        .mount("/", routes![index, new])
        .launch();
}


#[cfg(test)]
mod test {
    #[test]
    fn test() {}
}
