use std::fs::File;
use std::io::Write;

use actix_web::{get, HttpResponse, post, Responder, web};
use bytes::Bytes;
use serde::Deserialize;

#[derive(Deserialize)]
struct ObjectPath {
    bucket: String,
    object: String,
}

#[post("/{bucket}/{object}")]
pub async fn create_object(path: web::Path<ObjectPath>, payload: web::Payload) 
    -> Result<impl Responder, actix_web::Error>  {
    let bytes = payload.to_bytes().await.unwrap();
    let mut f = File::create_new(std::path::Path::new(&path.bucket).join(&path.object))?;
    f.write_all(&bytes)?;
    println!("{}", String::from_utf8(bytes.to_vec()).unwrap());
    Ok(HttpResponse::Created().finish())
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
