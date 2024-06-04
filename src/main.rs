mod controller;

use actix_web::{App, HttpServer};
use controller::object::{create_object, delete_object, read_object, update_object, version};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
        .service(version)
        .service(create_object)
        .service(read_object)
        .service(update_object)
        .service(delete_object)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}