use actix_web::{web, HttpResponse, Responder, Error, get, post, delete};
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Write};
use bytes::Bytes;
use crate::controller::versioning;

#[derive(Deserialize)]
struct ObjectPath {
    bucket: String,
    object: String,
}

impl ObjectPath {
    fn into_file(&self) -> Result<File, Error> {
        use std::path::Path;
        let f = File::create_new(Path::new(&self.bucket).join(&self.object))?;
        Ok(f)
    }    
}

#[post("/{bucket}/{object}")]
pub async fn create_object(path: web::Path<ObjectPath>, payload: web::Payload) 
    -> Result<impl Responder, Error>  {
    let bytes = payload.to_bytes().await?;
    let mut f = path.into_file()?;
    f.write_all(&bytes)?;
    
    // Create a new version
    let etag = format!("{:x}", md5::compute(&bytes));
    versioning::create_version(&path.bucket, &path.object, &bytes, &etag)?;
    
    Ok(HttpResponse::Created()
        .insert_header(("ETag", etag))
        .finish())
}

#[get("/{bucket}/{object}")]
pub async fn read_object(path: web::Path<ObjectPath>)
    -> Result<impl Responder, Error>  {
    let mut f = path.into_file()?;
    let mut content = Vec::new();
    f.read_to_end(&mut content)?;
    
    // Get the latest version
    if let Some(version) = versioning::VERSION_STORE.get_latest_version(&path.bucket, &path.object) {
        if version.is_delete_marker {
            return Err(actix_web::error::ErrorNotFound("Object is deleted"));
        }
    }
    
    Ok(HttpResponse::Ok().body(Bytes::from(content)))
}

#[get("/{bucket}/{object}")]
pub async fn update_object(_path: web::Path<ObjectPath>) -> impl Responder {
    // TODO: Implement logic to update object
    HttpResponse::NoContent().finish()
}

#[delete("/{bucket}/{object}")]
pub async fn delete_object(path: web::Path<ObjectPath>) -> Result<impl Responder, Error> {
    // Create a delete marker
    versioning::create_delete_marker(&path.bucket, &path.object)?;
    
    Ok(HttpResponse::NoContent().finish())
}
