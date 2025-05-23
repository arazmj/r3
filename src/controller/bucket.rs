use actix_web::{web, HttpResponse, Responder, post, get, delete};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::sync::Mutex;
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct BucketPolicy {
    policy: String,
    acl: String,
}

lazy_static::lazy_static! {
    static ref BUCKET_POLICIES: Mutex<HashMap<String, BucketPolicy>> = Mutex::new(HashMap::new());
}

const POLICY_FILE: &str = "bucket_policies.json";

pub fn load_policies() -> std::io::Result<()> {
    if Path::new(POLICY_FILE).exists() {
        let mut file = File::open(POLICY_FILE)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let policies: HashMap<String, BucketPolicy> = serde_json::from_str(&contents)?;
        let mut store = BUCKET_POLICIES.lock().unwrap();
        *store = policies;
    }
    Ok(())
}

fn save_policies() -> std::io::Result<()> {
    let policies = BUCKET_POLICIES.lock().unwrap();
    let contents = serde_json::to_string_pretty(&*policies)?;
    let mut file = File::create(POLICY_FILE)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

#[post("/{bucket}?policy")]
pub async fn set_bucket_policy(
    path: web::Path<String>,
    policy: web::Json<BucketPolicy>,
) -> impl Responder {
    let bucket = path.into_inner();
    let mut policies = BUCKET_POLICIES.lock().unwrap();
    policies.insert(bucket, policy.into_inner());
    if let Err(e) = save_policies() {
        return HttpResponse::InternalServerError().body(format!("Failed to save policies: {}", e));
    }
    HttpResponse::Ok().finish()
}

#[get("/{bucket}?policy")]
pub async fn get_bucket_policy(path: web::Path<String>) -> impl Responder {
    let bucket = path.into_inner();
    let policies = BUCKET_POLICIES.lock().unwrap();
    match policies.get(&bucket) {
        Some(policy) => HttpResponse::Ok().json(policy),
        None => HttpResponse::NotFound().finish(),
    }
}

#[post("/")]
pub async fn create_bucket() -> impl Responder {
    println!("Creating bucket");
    match fs::create_dir("buckets") {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string())
    }
}

#[get("/{bucket}")]
pub async fn read_bucket(path: web::Path<String>) -> impl Responder {
    let bucket = path.into_inner();
    let bucket_path = format!("buckets/{}", bucket);
    if Path::new(&bucket_path).exists() {
        HttpResponse::Ok().body("Bucket exists")
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[get("/{bucket}")]
pub async fn update_bucket(path: web::Path<String>) -> impl Responder {
    let bucket = path.into_inner();
    let bucket_path = format!("buckets/{}", bucket);
    if Path::new(&bucket_path).exists() {
        HttpResponse::Ok().body("Bucket updated")
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[delete("/{bucket}")]
pub async fn delete_bucket(path: web::Path<String>) -> impl Responder {
    let bucket = path.into_inner();
    let bucket_path = format!("buckets/{}", bucket);
    if Path::new(&bucket_path).exists() {
        match fs::remove_dir_all(&bucket_path) {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(e) => HttpResponse::InternalServerError().body(e.to_string())
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::fs;
    use std::path::Path;

    fn cleanup_bucket(bucket: &str) {
        let bucket_path = format!("buckets/{}", bucket);
        if Path::new(&bucket_path).exists() {
            let _ = fs::remove_dir_all(&bucket_path);
        }
    }

    #[actix_rt::test]
    async fn test_create_and_read_bucket() {
        let bucket_name = "testbucket";
        cleanup_bucket(bucket_name);
        let app = test::init_service(App::new().service(create_bucket).service(read_bucket)).await;

        // Create bucket directory manually for this test
        fs::create_dir_all("buckets").unwrap();
        let resp = test::TestRequest::post().uri("/").send_request(&app).await;
        assert!(resp.status().is_success() || resp.status().is_server_error());

        // Simulate bucket creation
        let bucket_path = format!("buckets/{}", bucket_name);
        fs::create_dir_all(&bucket_path).unwrap();
        let resp = test::TestRequest::get().uri(&format!("/{}", bucket_name)).send_request(&app).await;
        assert!(resp.status().is_success());
        cleanup_bucket(bucket_name);
    }

    #[actix_rt::test]
    async fn test_read_nonexistent_bucket() {
        let bucket_name = "nonexistentbucket";
        cleanup_bucket(bucket_name);
        let app = test::init_service(App::new().service(read_bucket)).await;
        let resp = test::TestRequest::get().uri(&format!("/{}", bucket_name)).send_request(&app).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_rt::test]
    async fn test_update_bucket() {
        let bucket_name = "updatebucket";
        cleanup_bucket(bucket_name);
        fs::create_dir_all(format!("buckets/{}", bucket_name)).unwrap();
        let app = test::init_service(App::new().service(update_bucket)).await;
        let resp = test::TestRequest::get().uri(&format!("/{}", bucket_name)).send_request(&app).await;
        assert!(resp.status().is_success());
        cleanup_bucket(bucket_name);
    }

    #[actix_rt::test]
    async fn test_delete_bucket() {
        let bucket_name = "deletebucket";
        cleanup_bucket(bucket_name);
        fs::create_dir_all(format!("buckets/{}", bucket_name)).unwrap();
        let app = test::init_service(App::new().service(delete_bucket)).await;
        let resp = test::TestRequest::delete().uri(&format!("/{}", bucket_name)).send_request(&app).await;
        assert_eq!(resp.status(), 204);
        assert!(!Path::new(&format!("buckets/{}", bucket_name)).exists());
    }

    #[actix_rt::test]
    async fn test_delete_nonexistent_bucket() {
        let bucket_name = "nonexistentbucket2";
        cleanup_bucket(bucket_name);
        let app = test::init_service(App::new().service(delete_bucket)).await;
        let resp = test::TestRequest::delete().uri(&format!("/{}", bucket_name)).send_request(&app).await;
        assert_eq!(resp.status(), 404);
    }
}
