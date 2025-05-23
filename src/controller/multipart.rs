use actix_web::{web, HttpResponse, Responder, Error, post, put, delete};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;

// Structure to store upload information
#[derive(Debug, Serialize, Deserialize, Clone)]
struct UploadInfo {
    upload_id: String,
    bucket: String,
    key: String,
    parts: HashMap<u32, String>, // part number -> etag
    created_at: u64,
}

// In-memory storage for upload information
struct UploadStore {
    uploads: Mutex<HashMap<String, UploadInfo>>, // upload_id -> UploadInfo
}

impl UploadStore {
    fn new() -> Self {
        UploadStore {
            uploads: Mutex::new(HashMap::new()),
        }
    }

    fn create_upload(&self, bucket: String, key: String) -> String {
        let upload_id = format!("{:x}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos());
        
        let upload_info = UploadInfo {
            upload_id: upload_id.clone(),
            bucket,
            key,
            parts: HashMap::new(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.uploads.lock().unwrap().insert(upload_id.clone(), upload_info);
        upload_id
    }

    fn get_upload(&self, upload_id: &str) -> Option<UploadInfo> {
        self.uploads.lock().unwrap().get(upload_id).cloned()
    }

    fn add_part(&self, upload_id: &str, part_number: u32, etag: String) -> bool {
        if let Some(upload) = self.uploads.lock().unwrap().get_mut(upload_id) {
            upload.parts.insert(part_number, etag);
            true
        } else {
            false
        }
    }
}

// Initialize upload store
lazy_static! {
    static ref UPLOAD_STORE: UploadStore = UploadStore::new();
}

// Endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMultipartUploadResponse {
    bucket: String,
    key: String,
    upload_id: String,
}

#[post("/{bucket}/{key}")]
pub async fn initiate_multipart_upload(
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    let (bucket, key) = path.into_inner();
    
    if !query.contains_key("uploads") {
        return Ok(HttpResponse::BadRequest().body("Missing uploads parameter"));
    }

    let upload_id = uuid::Uuid::new_v4().to_string();
    let response = InitiateMultipartUploadResponse {
        bucket,
        key,
        upload_id: upload_id.clone(),
    };

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <InitiateMultipartUploadResult>
            <Bucket>{}</Bucket>
            <Key>{}</Key>
            <UploadId>{}</UploadId>
        </InitiateMultipartUploadResult>"#,
        response.bucket, response.key, response.upload_id
    );

    Ok(HttpResponse::Ok()
        .content_type("application/xml")
        .body(xml))
}

#[put("/{bucket}/{key}")]
pub async fn upload_part(
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
    payload: web::Payload,
) -> Result<impl Responder, Error> {
    let (bucket, _key) = path.into_inner();
    let upload_id = query.get("uploadId").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Missing uploadId parameter")
    })?;
    let part_number = query.get("partNumber")
        .and_then(|p| p.parse::<u32>().ok())
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest("Invalid partNumber parameter")
        })?;

    // Create directory for multipart upload if it doesn't exist
    let upload_dir = format!("{}/{}", bucket, upload_id);
    fs::create_dir_all(&upload_dir)?;

    // Save the part
    let part_path = format!("{}/part-{}", upload_dir, part_number);
    let mut file = File::create(&part_path)?;
    let bytes = payload.to_bytes().await?;
    file.write_all(&bytes)?;

    // Generate ETag (simplified version)
    let etag = format!("{:x}", md5::compute(&bytes));
    UPLOAD_STORE.add_part(upload_id, part_number, etag.clone());

    Ok(HttpResponse::Ok()
        .insert_header(("ETag", etag))
        .finish())
}

#[post("/{bucket}/{key}/complete")]
pub async fn complete_multipart_upload(
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    let (bucket, _key) = path.into_inner();
    let upload_id = query.get("uploadId").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Missing uploadId parameter")
    })?;

    println!("[DEBUG] complete_multipart_upload: upload_id = {}", upload_id);

    let upload_info = match UPLOAD_STORE.get_upload(upload_id) {
        Some(info) => info,
        None => {
            println!("[DEBUG] complete_multipart_upload: upload_info not found for upload_id = {}", upload_id);
            return Err(actix_web::error::ErrorNotFound("Upload not found"));
        }
    };

    // Check if all parts are present
    let expected_parts: Vec<u32> = (1..=upload_info.parts.len() as u32).collect();
    let mut part_numbers: Vec<u32> = upload_info.parts.keys().cloned().collect();
    part_numbers.sort();
    println!("[DEBUG] complete_multipart_upload: part_numbers = {:?}, expected_parts = {:?}", part_numbers, expected_parts);
    if part_numbers != expected_parts {
        println!("[DEBUG] complete_multipart_upload: Not all parts are present");
        return Err(actix_web::error::ErrorBadRequest("Not all parts are present"));
    }

    // Combine all parts
    let upload_dir = format!("{}/{}", bucket, upload_id);
    println!("[DEBUG] complete_multipart_upload: upload_dir = {}", upload_dir);
    let final_path = format!("{}/{}", bucket, _key);
    let mut final_file = match File::create(&final_path) {
        Ok(f) => f,
        Err(e) => {
            println!("[DEBUG] complete_multipart_upload: Failed to create final file: {}", e);
            return Err(actix_web::error::ErrorInternalServerError("Failed to create final file"));
        }
    };

    for part_number in &part_numbers {
        let part_path = format!("{}/part-{}", upload_dir, part_number);
        println!("[DEBUG] complete_multipart_upload: reading part_path = {}", part_path);
        let mut part_file = match File::open(&part_path) {
            Ok(f) => f,
            Err(e) => {
                println!("[DEBUG] complete_multipart_upload: Failed to open part file {}: {}", part_path, e);
                return Err(actix_web::error::ErrorInternalServerError("Failed to open part file"));
            }
        };
        if let Err(e) = std::io::copy(&mut part_file, &mut final_file) {
            println!("[DEBUG] complete_multipart_upload: Failed to copy part file {}: {}", part_path, e);
            return Err(actix_web::error::ErrorInternalServerError("Failed to copy part file"));
        }
    }

    // Clean up temporary files
    if let Err(e) = fs::remove_dir_all(&upload_dir) {
        println!("[DEBUG] complete_multipart_upload: Failed to remove upload_dir {}: {}", upload_dir, e);
    }

    let response = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
        <CompleteMultipartUploadResult>
            <Location>http://localhost:8080/{}/{}</Location>
            <Bucket>{}</Bucket>
            <Key>{}</Key>
            <ETag>\"{}\"</ETag>
        </CompleteMultipartUploadResult>"#,
        bucket, _key, bucket, _key, upload_id
    );
    println!("[DEBUG] complete_multipart_upload response: {}", response);

    Ok(HttpResponse::Ok()
        .content_type("application/xml")
        .body(response))
}

#[delete("/{bucket}/{key}")]
pub async fn abort_multipart_upload(
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    let (bucket, _key) = path.into_inner();
    let upload_id = query.get("uploadId").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Missing uploadId parameter")
    })?;

    // Remove temporary files
    let upload_dir = format!("{}/{}", bucket, upload_id);
    if Path::new(&upload_dir).exists() {
        fs::remove_dir_all(&upload_dir)?;
    }

    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::fs;
    use std::path::Path;
    use bytes::Bytes;

    fn setup_bucket(bucket: &str) {
        let bucket_path = Path::new(bucket);
        if !bucket_path.exists() {
            fs::create_dir_all(bucket_path).unwrap();
        }
    }

    fn cleanup_upload(bucket: &str, upload_id: &str) {
        let upload_dir = format!("{}/{}", bucket, upload_id);
        if Path::new(&upload_dir).exists() {
            let _ = fs::remove_dir_all(&upload_dir);
        }
        if Path::new(bucket).exists() {
            let _ = fs::remove_dir_all(bucket);
        }
    }

    fn extract_upload_id(body_str: &str) -> Option<String> {
        body_str
            .split("<UploadId>")
            .nth(1)?
            .split("</UploadId>")
            .next()
            .map(|s| s.trim().to_string())
    }

    #[actix_rt::test]
    async fn test_initiate_multipart_upload() {
        let bucket = "testbucket_mp";
        let key = "testfile.txt";
        setup_bucket(bucket);
        let app = test::init_service(App::new().service(initiate_multipart_upload)).await;

        let req = test::TestRequest::post()
            .uri(&format!("/{}/{}?uploads", bucket, key))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        if status != 200 {
            println!("test_initiate_multipart_upload: status = {:?}, body = {}", status, body_str);
        }
        assert_eq!(status, 200);
        assert!(body_str.contains("<InitiateMultipartUploadResult>"));
        assert!(body_str.contains(&format!("<Bucket>{}</Bucket>", bucket)));
        assert!(body_str.contains(&format!("<Key>{}</Key>", key)));
        assert!(body_str.contains("<UploadId>"));
        cleanup_upload(bucket, "");
    }

    #[actix_rt::test]
    async fn test_upload_part() {
        let bucket = "testbucket_mp2";
        let key = "testfile2.txt";
        setup_bucket(bucket);
        let app = test::init_service(App::new()
            .service(initiate_multipart_upload)
            .service(upload_part))
            .await;

        // First initiate upload
        let req = test::TestRequest::post()
            .uri(&format!("/{}/{}?uploads", bucket, key))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        if status != 200 {
            println!("test_upload_part (initiate): status = {:?}, body = {}", status, body_str);
        }
        assert_eq!(status, 200);
        let upload_id = extract_upload_id(&body_str).expect("Failed to extract upload ID");
        assert!(!upload_id.is_empty(), "Upload ID should not be empty");

        // Upload a part
        let data = Bytes::from_static(b"test part data");
        let req = test::TestRequest::put()
            .uri(&format!("/{}/{}?uploadId={}&partNumber=1", bucket, key, upload_id))
            .set_payload(data)
            .to_request();
        let resp = test::call_service(&app, req).await;
        let etag_header = resp.headers().get("ETag").cloned();
        let status = resp.status();
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        if status != 200 {
            println!("test_upload_part (upload): status = {:?}, body = {}", status, body_str);
        }
        assert_eq!(status, 200);
        assert!(etag_header.is_some());
        cleanup_upload(bucket, &upload_id);
    }

    #[actix_rt::test]
    async fn test_abort_multipart_upload() {
        let bucket = "testbucket_mp4";
        let key = "testfile4.txt";
        setup_bucket(bucket);
        let app = test::init_service(App::new()
            .service(initiate_multipart_upload)
            .service(upload_part)
            .service(abort_multipart_upload))
            .await;

        // Initiate upload
        let req = test::TestRequest::post()
            .uri(&format!("/{}/{}?uploads", bucket, key))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        if status != 200 {
            println!("test_abort_multipart_upload (initiate): status = {:?}, body = {}", status, body_str);
        }
        assert_eq!(status, 200);
        let upload_id = extract_upload_id(&body_str).expect("Failed to extract upload ID");
        assert!(!upload_id.is_empty(), "Upload ID should not be empty");

        // Upload a part
        let data = Bytes::from_static(b"test part data");
        let req = test::TestRequest::put()
            .uri(&format!("/{}/{}?uploadId={}&partNumber=1", bucket, key, upload_id))
            .set_payload(data)
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        if status != 200 {
            println!("test_abort_multipart_upload (upload): status = {:?}, body = {}", status, body_str);
        }
        assert_eq!(status, 200);

        // Abort upload
        let req = test::TestRequest::delete()
            .uri(&format!("/{}/{}?uploadId={}", bucket, key, upload_id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        if status != 204 {
            println!("test_abort_multipart_upload (abort): status = {:?}, body = {}", status, body_str);
        }
        assert_eq!(status, 204);
        let upload_dir = format!("{}/{}", bucket, upload_id);
        assert!(!Path::new(&upload_dir).exists());
        cleanup_upload(bucket, &upload_id);
    }

    #[actix_rt::test]
    async fn test_upload_part_missing_upload_id() {
        let bucket = "testbucket_mp5";
        let key = "testfile5.txt";
        setup_bucket(bucket);
        let app = test::init_service(App::new().service(upload_part)).await;

        let data = Bytes::from_static(b"test part data");
        let req = test::TestRequest::put()
            .uri(&format!("/{}/{}?partNumber=1", bucket, key))
            .set_payload(data)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
        
        cleanup_upload(bucket, "");
    }

    #[actix_rt::test]
    async fn test_upload_part_invalid_part_number() {
        let bucket = "testbucket_mp6";
        let key = "testfile6.txt";
        setup_bucket(bucket);
        let app = test::init_service(App::new().service(upload_part)).await;

        let data = Bytes::from_static(b"test part data");
        let req = test::TestRequest::put()
            .uri(&format!("/{}/{}?uploadId=123&partNumber=invalid", bucket, key))
            .set_payload(data)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
        
        cleanup_upload(bucket, "");
    }
} 