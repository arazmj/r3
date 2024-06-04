use actix_web::{get, web, HttpResponse, Responder};
use bytes::Bytes;
use serde::Deserialize;


#[derive(Deserialize)]
struct ObjectPath {
    bucket: String,
    object: String,
}

#[get("/{bucket}/{object}")]
pub async fn create_object(_path: web::Path<ObjectPath>) -> impl Responder {
    // TODO: Implement logic to create object

    HttpResponse::Created().finish()
}

#[get("/{bucket}/{object}")]
pub async fn read_object(_path: web::Path<ObjectPath>) -> impl Responder {
    // TODO: Implement logic to read object
    HttpResponse::Ok().body(Bytes::from_static(b"Object data"))
}

#[get("/{bucket}/{object}")]
pub async fn update_object(_path: web::Path<ObjectPath>) -> impl Responder {
    // TODO: Implement logic to update object
    HttpResponse::NoContent().finish()
}

#[get("/{bucket}/{object}")]
pub async fn delete_object(_path: web::Path<ObjectPath>) -> impl Responder {
    // TODO: Implement logic to delete object
    HttpResponse::NoContent().finish()
}


#[get("/")]
pub async fn version() -> impl Responder {
    "Hello, world!"
}