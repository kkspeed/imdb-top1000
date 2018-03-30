#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate crawler;
extern crate rocket;
extern crate serde_json;

use crawler::{ItemType, ModelType, Storage};

fn main() {
    let crawler = crawler::Crawler::new(8);
    match crawler.crawl() {
        Ok(map) => {
            rocket::ignite()
                .manage(map)
                .mount("/", routes![get])
                .launch();
        }
        Err(err) => println!("Error: {:?}", err),
    }
}

#[get("/<query_str>")]
fn get(model: rocket::State<ModelType>, query_str: String) -> String {
    serde_json::to_string(&model
        .query(query_str.to_lowercase().as_str())
        .iter()
        .map(&|s: &ItemType| s.as_json_string())
        .collect::<Vec<String>>()).unwrap()
}
