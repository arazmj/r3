use actix_web::{App, HttpServer};

use controller::bucket;
use controller::object;
use controller::multipart;
use controller::versioning;
use controller::version;

mod controller;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(r#"
  ____  ____  _____
 |  _ \|  _ \| ____|
 | |_) | |_) |  _|  
 |  _ <|  _ <| |___ 
 |_| \_\_| \_\_____|

S3-Compatible Storage Service
Author: Your Name
Version: 0.1.0
Listening on: http://127.0.0.1:8080
"#);
    HttpServer::new(|| {
        App::new()
            .service(version)
            .service(object::create_object)
            .service(object::read_object)
            .service(object::delete_object)
            .service(bucket::create_bucket)
            .service(bucket::delete_bucket)
            .service(multipart::initiate_multipart_upload)
            .service(multipart::upload_part)
            .service(multipart::complete_multipart_upload)
            .service(multipart::abort_multipart_upload)
            .service(versioning::put_bucket_versioning)
            .service(versioning::list_object_versions)
            .service(versioning::get_object_version)
            .service(versioning::delete_object_version)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}