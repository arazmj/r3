use actix_web::{web, HttpResponse, Responder, Error, get, put, delete};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use bytes::Bytes;
use lazy_static::lazy_static;

// Structure to store version information
#[derive(Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version_id: String,
    pub bucket: String,
    pub key: String,
    pub is_latest: bool,
    pub is_delete_marker: bool,
    pub size: u64,
    pub last_modified: u64,
    pub etag: String,
}

// In-memory storage for version information
pub struct VersionStore {
    versions: Mutex<HashMap<String, HashMap<String, Vec<VersionInfo>>>>,
}

impl VersionStore {
    pub fn new() -> Self {
        VersionStore {
            versions: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_version(&self, bucket: &str, key: &str, version_info: VersionInfo) {
        let mut versions = self.versions.lock().unwrap();
        let bucket_map = versions.entry(bucket.to_string()).or_insert_with(HashMap::new);
        let version_list = bucket_map.entry(key.to_string()).or_insert_with(Vec::new);
        // Mark all other versions as not latest
        for v in version_list.iter_mut() {
            v.is_latest = false;
        }
        version_list.push(version_info);
    }

    pub fn get_versions(&self, bucket: &str, key: &str) -> Vec<VersionInfo> {
        self.versions.lock().unwrap()
            .get(bucket)
            .and_then(|bucket_map| bucket_map.get(key))
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_version(&self, bucket: &str, key: &str, version_id: &str) -> Option<VersionInfo> {
        self.versions.lock().unwrap()
            .get(bucket)
            .and_then(|bucket_map| bucket_map.get(key))
            .and_then(|versions| versions.iter().find(|v| v.version_id == version_id).cloned())
    }

    pub fn get_latest_version(&self, bucket: &str, key: &str) -> Option<VersionInfo> {
        self.versions
            .lock().unwrap()
            .get(bucket)
            .and_then(|keys| keys.get(key))
            .and_then(|versions| versions.last().cloned())
    }
}

// Initialize version store
lazy_static! {
    pub static ref VERSION_STORE: VersionStore = VersionStore::new();
}

// Helper function to generate version ID
fn generate_version_id() -> String {
    format!("{:x}", SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos())
}

// Helper function to get current timestamp
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// Endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct VersioningConfiguration {
    status: String,
}

#[put("/{bucket}")]
pub async fn put_bucket_versioning(
    path: web::Path<String>,
    _config: web::Json<VersioningConfiguration>,
) -> Result<HttpResponse, Error> {
    let bucket = path.into_inner();
    let mut store = VERSION_STORE.versions.lock().unwrap();
    store.insert(bucket, HashMap::new());
    Ok(HttpResponse::Ok().finish())
}

#[get("/{bucket}")]
pub async fn get_bucket_versioning(
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let bucket = path.into_inner();
    let store = VERSION_STORE.versions.lock().unwrap();
    let has_versions = store.contains_key(&bucket);
    let response = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
        <VersioningConfiguration>
            <Status>{}</Status>
        </VersioningConfiguration>"#,
        if has_versions { "Enabled" } else { "Suspended" }
    );
    Ok(HttpResponse::Ok()
        .content_type("application/xml")
        .body(response))
}

#[get("/{bucket}/versions")]
pub async fn list_object_versions(
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    let bucket = path.into_inner();
    let versions = VERSION_STORE.get_versions(&bucket, query.get("key").unwrap_or(&String::new()));
    let response = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
        <ListVersionsResult>
            <Name>{}</Name>
            <Prefix>{}</Prefix>
            <KeyMarker></KeyMarker>
            <VersionIdMarker></VersionIdMarker>
            <MaxKeys>1000</MaxKeys>
            <IsTruncated>false</IsTruncated>
            {}
        </ListVersionsResult>"#,
        bucket,
        query.get("prefix").unwrap_or(&String::new()),
        versions.iter().map(|v| format!(
            r#"<Version>
                <Key>{}</Key>
                <VersionId>{}</VersionId>
                <IsLatest>{}</IsLatest>
                <LastModified>{}</LastModified>
                <ETag>\"{}\"</ETag>
                <Size>{}</Size>
                <StorageClass>STANDARD</StorageClass>
            </Version>"#,
            v.key,
            v.version_id,
            v.is_latest,
            v.last_modified,
            v.etag,
            v.size
        )).collect::<Vec<String>>().join("\n")
    );
    Ok(HttpResponse::Ok()
        .content_type("application/xml")
        .body(response))
}

#[get("/{bucket}/{key}?versions")]
pub async fn list_object_versions_old(
    path: web::Path<(String, String)>,
) -> Result<impl Responder, Error> {
    let (bucket, key) = path.into_inner();
    let versions = VERSION_STORE.get_versions(&bucket, &key);

    let response = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <ListVersionsResult>
            <Name>{}</Name>
            <Prefix>{}</Prefix>
            <KeyMarker></KeyMarker>
            <VersionIdMarker></VersionIdMarker>
            <MaxKeys>1000</MaxKeys>
            <IsTruncated>false</IsTruncated>
            {}
        </ListVersionsResult>"#,
        bucket,
        key,
        versions.iter().map(|v| format!(
            r#"<Version>
                <Key>{}</Key>
                <VersionId>{}</VersionId>
                <IsLatest>{}</IsLatest>
                <LastModified>{}</LastModified>
                <ETag>"{}"</ETag>
                <Size>{}</Size>
                <StorageClass>STANDARD</StorageClass>
                <Owner>
                    <ID>owner</ID>
                    <DisplayName>owner</DisplayName>
                </Owner>
            </Version>"#,
            v.key,
            v.version_id,
            v.is_latest,
            v.last_modified,
            v.etag,
            v.size
        )).collect::<Vec<String>>().join("\n")
    );

    Ok(HttpResponse::Ok()
        .content_type("application/xml")
        .body(response))
}

#[get("/{bucket}/{key}?versionId={version_id}")]
pub async fn get_object_version(
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    let (bucket, key) = path.into_inner();
    let version_id = query.get("versionId").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Missing versionId parameter")
    })?;

    let version = VERSION_STORE.get_version(&bucket, &key, version_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound("Version not found"))?;

    if version.is_delete_marker {
        return Err(actix_web::error::ErrorNotFound("Version is a delete marker"));
    }

    let version_path = format!("{}/.versions/{}/{}", bucket, version_id, key);
    let mut file = File::open(&version_path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    Ok(HttpResponse::Ok()
        .insert_header(("ETag", version.etag))
        .insert_header(("Last-Modified", version.last_modified.to_string()))
        .body(Bytes::from(content)))
}

#[delete("/{bucket}/{key}?versionId={version_id}")]
pub async fn delete_object_version(
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    let (bucket, key) = path.into_inner();
    let version_id = query.get("versionId").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Missing versionId parameter")
    })?;

    let version_path = format!("{}/.versions/{}/{}", bucket, version_id, key);
    if Path::new(&version_path).exists() {
        fs::remove_file(&version_path)?;
        // Remove version from in-memory store
        let mut versions = VERSION_STORE.versions.lock().unwrap();
        if let Some(bucket_map) = versions.get_mut(&bucket) {
            if let Some(version_list) = bucket_map.get_mut(&key) {
                version_list.retain(|v| v.version_id != *version_id);
            }
        }
    }

    Ok(HttpResponse::NoContent().finish())
}

// Helper function to create a new version
pub fn create_version(bucket: &str, key: &str, content: &[u8], etag: &str) -> Result<(), Error> {
    let version_id = generate_version_id();
    let version_path = format!("{}/.versions/{}/{}", bucket, version_id, key);
    
    // Check if version directory exists
    if Path::new(&version_path).exists() {
        return Err(actix_web::error::ErrorBadRequest("Version already exists"));
    }
    
    // Create version directory
    fs::create_dir_all(Path::new(&version_path).parent().unwrap())?;
    
    // Write version content
    let mut file = File::create(&version_path)?;
    file.write_all(content)?;

    // Create version info
    let version_info = VersionInfo {
        version_id: version_id.clone(),
        bucket: bucket.to_string(),
        key: key.to_string(),
        is_latest: true,
        is_delete_marker: false,
        size: content.len() as u64,
        last_modified: get_current_timestamp(),
        etag: etag.to_string(),
    };

    VERSION_STORE.add_version(bucket, key, version_info);
    Ok(())
}

// Helper function to create a delete marker
pub fn create_delete_marker(bucket: &str, key: &str) -> Result<(), Error> {
    let version_id = generate_version_id();
    
    let version_info = VersionInfo {
        version_id: version_id.clone(),
        bucket: bucket.to_string(),
        key: key.to_string(),
        is_latest: true,
        is_delete_marker: true,
        size: 0,
        last_modified: get_current_timestamp(),
        etag: "".to_string(),
    };

    VERSION_STORE.add_version(bucket, key, version_info);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_put_bucket_versioning() {
        let app = test::init_service(
            actix_web::App::new()
                .service(put_bucket_versioning)
        ).await;

        let req = test::TestRequest::put()
            .uri("/test-bucket")
            .set_json(&VersioningConfiguration {
                status: "Enabled".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_get_bucket_versioning() {
        let app = test::init_service(
            actix_web::App::new()
                .service(get_bucket_versioning)
        ).await;

        let req = test::TestRequest::get()
            .uri("/test-bucket")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
} 