use actix_web::{web, HttpResponse, Responder, post, get, delete};
use std::fs;

#[post("/")]
pub async fn create_bucket() -> impl Responder {
    println!("Creating bucket");
    match fs::create_dir("path") {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string())
    }
}

#[get("/{bucket}")]
pub async fn read_bucket(_path: web::Path<String>) -> impl Responder {
    // TODO: Implement logic to read object
    HttpResponse::Ok().body("Object data")
}

#[get("/{bucket}")]
pub async fn update_bucket(_path: web::Path<String>) -> impl Responder {
    // TODO: Implement logic to update object
    HttpResponse::NoContent().finish()
}

#[delete("/{bucket}")]
pub async fn delete_bucket(_path: web::Path<String>) -> impl Responder {
    // TODO: Implement logic to delete object
    HttpResponse::NoContent().finish()
}
