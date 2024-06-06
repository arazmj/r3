pub mod object;
pub mod bucket;

use actix_web::{Responder, get};


#[get("/")]
pub async fn version() -> impl Responder {
    "Hello, world!"
}