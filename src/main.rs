use actix_web::{App, HttpServer};

use controller::bucket;
use controller::object;
use controller::multipart;
use controller::versioning;
use controller::version;
use controller::auth;

mod controller;

const SERVER_ADDRESS: &str = "127.0.0.1:8080";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = bucket::load_policies() {
        eprintln!("Failed to load bucket policies: {}", e);
    }
    println!(r#"
  ____  ____  _____
 |  _ \|  _ \| ____|
 | |_) | |_) |  _|  
 |  _ <|  _ <| |___ 
 |_| \_\_| \_\_____|

S3-Compatible Storage Service
Author: Your Name
Version: 0.1.0
Listening on: http://{}
"#, SERVER_ADDRESS);
    HttpServer::new(|| {
        App::new()
            .service(version)
            .service(object::create_object)
            .service(object::read_object)
            .service(object::delete_object)
            .service(bucket::create_bucket)
            .service(bucket::delete_bucket)
            .service(bucket::set_bucket_policy)
            .service(bucket::get_bucket_policy)
            .service(multipart::initiate_multipart_upload)
            .service(multipart::upload_part)
            .service(multipart::complete_multipart_upload)
            .service(multipart::abort_multipart_upload)
            .service(versioning::put_bucket_versioning)
            .service(versioning::list_object_versions)
            .service(versioning::get_object_version)
            .service(versioning::delete_object_version)
            .service(auth::register)
            .service(auth::login)
    })
    .bind(SERVER_ADDRESS)?
    .run()
    .await
}