use actix_web::{App, HttpServer};

use controller::bucket;
use controller::object;

use crate::controller::version;

mod controller;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
        .service(version)
        .service(object::create_object)
        .service(object::read_object)
        .service(object::update_object)
        .service(object::delete_object)
        .service(bucket::create_bucket)
        .service(bucket::delete_bucket)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}