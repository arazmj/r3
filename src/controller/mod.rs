pub mod object;

use actix_web::{Responder, get};



#[get("/")]
pub async fn version() -> impl Responder {
    "Hello, world!"
}