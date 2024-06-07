use std::fs::File;
use std::io::{Read, Write};

use actix_web::{Error, get, HttpResponse, post, Responder, web};
use serde::Deserialize;

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
    -> Result<impl Responder, actix_web::Error>  {
    let bytes = payload.to_bytes().await.unwrap();
    let mut f = &path.into_file()?;
    f.write_all(&bytes)?;
    println!("{}", String::from_utf8(bytes.to_vec()).unwrap());
    Ok(HttpResponse::Created().finish())
}

#[get("/{bucket}/{object}")]
pub async fn read_object(path: web::Path<ObjectPath>)
    -> Result<impl Responder, actix_web::Error>  {
    let mut f = &path.into_file()?;
    let mut content =  vec![];
    f.read_to_end(&mut content)?;
    Ok(HttpResponse::Ok().streaming(content))
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
