use actix_web::{web, HttpResponse, Responder};
use actix_web::post;
use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

lazy_static! {
    static ref USER_STORE: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

#[post("/register")]
pub async fn register(user: web::Json<User>) -> impl Responder {
    println!("Registering user: {}", user.username);
    let mut store = USER_STORE.lock().unwrap();
    if store.contains_key(&user.username) {
        println!("User {} already exists", user.username);
        return HttpResponse::Conflict().json("User already exists");
    }

    match hash(&user.password, DEFAULT_COST) {
        Ok(hashed) => {
            store.insert(user.username.clone(), hashed);
            println!("Successfully registered user: {}", user.username);
            HttpResponse::Created().json("User registered successfully")
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to hash password")
    }
}

#[post("/login")]
pub async fn login(user: web::Json<User>) -> impl Responder {
    let store = USER_STORE.lock().unwrap();
    match store.get(&user.username) {
        Some(hashed) => {
            match verify(&user.password, hashed) {
                Ok(true) => HttpResponse::Ok().json("Login successful"),
                _ => HttpResponse::Unauthorized().json("Invalid credentials")
            }
        }
        None => HttpResponse::Unauthorized().json("User not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use serial_test::serial;

    fn clear_store() {
        println!("Clearing entire user store");
        let mut store = USER_STORE.lock().unwrap();
        store.clear();
    }

    #[actix_web::test]
    #[serial]
    async fn test_register() {
        let username = "testuser";
        clear_store();
        
        let app = test::init_service(
            actix_web::App::new()
                .service(register)
        ).await;

        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(&User {
                username: username.to_string(),
                password: "testpass".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        println!("Register response status: {}", resp.status());
        assert_eq!(resp.status(), 201); // Expect Created status
        let body = test::read_body(resp).await;
        println!("Register response body: {:?}", body);
        
        clear_store();
    }

    #[actix_web::test]
    #[serial]
    async fn test_login() {
        let username = "testuser";
        let password = "testpass";
        clear_store();
        
        let app = test::init_service(
            actix_web::App::new()
                .service(register)
                .service(login)
        ).await;

        // Register a user first
        let register_req = test::TestRequest::post()
            .uri("/register")
            .set_json(&User {
                username: username.to_string(),
                password: password.to_string(),
            })
            .to_request();
        let resp = test::call_service(&app, register_req).await;
        assert_eq!(resp.status(), 201); // Expect Created status

        // Now test login
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(&User {
                username: username.to_string(),
                password: password.to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        println!("Login response status: {}", resp.status());
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        println!("Login response body: {:?}", body);
        
        clear_store();
    }
} 