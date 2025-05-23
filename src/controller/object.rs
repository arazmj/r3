use actix_web::{web, HttpResponse, Responder, Error, get, post, delete};
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Write};
use bytes::Bytes;
use crate::controller::versioning;
use std::path::Path;

#[derive(Deserialize)]
struct ObjectPath {
    bucket: String,
    object: String,
}

impl ObjectPath {
    fn into_file(&self) -> Result<File, Error> {
        use std::path::Path;
        let path = Path::new(&self.bucket).join(&self.object);
        if path.exists() {
            File::open(path).map_err(|e| actix_web::error::ErrorInternalServerError(e))
        } else {
            File::create(path).map_err(|e| actix_web::error::ErrorInternalServerError(e))
        }
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
    let file_path = Path::new(&path.bucket).join(&path.object);
    if !file_path.exists() {
        return Err(actix_web::error::ErrorNotFound("Object not found"));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use bytes::Bytes;

    fn setup_bucket(bucket: &str) {
        let bucket_path = Path::new(bucket);
        if !bucket_path.exists() {
            fs::create_dir_all(bucket_path).unwrap();
        }
    }

    fn cleanup_object(bucket: &str, object: &str) {
        let object_path = Path::new(bucket).join(object);
        if object_path.exists() {
            let _ = fs::remove_file(&object_path);
        }
        if Path::new(bucket).exists() {
            let _ = fs::remove_dir_all(bucket);
        }
    }

    #[actix_rt::test]
    async fn test_create_and_read_object() {
        let bucket = "testbucket_obj";
        let object = "testobject.txt";
        cleanup_object(bucket, object);
        setup_bucket(bucket);
        let app = test::init_service(App::new().service(create_object).service(read_object)).await;

        // Create object
        let data = Bytes::from_static(b"Hello, world!");
        let req = test::TestRequest::post()
            .uri(&format!("/{}/{}", bucket, object))
            .set_payload(data.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        // Read object
        let req = test::TestRequest::get()
            .uri(&format!("/{}/{}", bucket, object))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body = test::read_body(resp).await;
        assert_eq!(body, data);
        cleanup_object(bucket, object);
    }

    #[actix_rt::test]
    async fn test_read_nonexistent_object() {
        let bucket = "testbucket_obj2";
        let object = "nonexistent.txt";
        cleanup_object(bucket, object);
        setup_bucket(bucket);
        let app = test::init_service(App::new().service(read_object)).await;
        let req = test::TestRequest::get()
            .uri(&format!("/{}/{}", bucket, object))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
        cleanup_object(bucket, object);
    }

    #[actix_rt::test]
    async fn test_delete_object() {
        let bucket = "testbucket_obj3";
        let object = "delete_me.txt";
        cleanup_object(bucket, object);
        setup_bucket(bucket);
        // Create the object file
        let object_path = Path::new(bucket).join(object);
        let mut file = fs::File::create(&object_path).unwrap();
        file.write_all(b"delete me").unwrap();
        let app = test::init_service(App::new().service(delete_object).service(read_object)).await;
        // Delete object
        let req = test::TestRequest::delete()
            .uri(&format!("/{}/{}", bucket, object))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 204);
        // Try to read deleted object
        let req = test::TestRequest::get()
            .uri(&format!("/{}/{}", bucket, object))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
        cleanup_object(bucket, object);
    }
}
