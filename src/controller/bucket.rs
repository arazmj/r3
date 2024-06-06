use std::fs;

use actix_web::{delete, error, HttpResponse, post, Responder, web};
use actix_web::http::StatusCode;
use guid_create::GUID;

#[post("/bucket")]
pub async fn create_bucket() -> Result<impl Responder, actix_web::Error> {
    println!("Creating bucket");
    let guid = GUID::rand().to_string();
    fs::create_dir(&guid).map_err(|e| error::InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(HttpResponse::Created().body(guid))
}

#[delete("/bucket/{bucket}")]
pub async fn delete_bucket(path: web::Path<String>) ->  Result<impl Responder, actix_web::Error> {
    println!("Deleting bucket {}", path.clone().to_uppercase());
    fs::remove_dir(path.into_inner().to_uppercase()).map_err(|e| error::InternalError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(HttpResponse::NoContent().finish())
}
