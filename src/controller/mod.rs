pub mod object;
pub mod bucket;
pub mod multipart;
pub mod versioning;
pub mod auth;

use actix_web::{Responder, get};


#[get("/")]
pub async fn version() -> impl Responder {
    "Hello, world!"
}