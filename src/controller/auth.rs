use actix_web::{web, HttpResponse, Error, post};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use bcrypt::{hash, verify, DEFAULT_COST};


lazy_static! {
    static ref USER_STORE: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    token: String,
}

#[post("/register")]
pub async fn register(user: web::Json<User>) -> Result<HttpResponse, Error> {
    let mut store = USER_STORE.lock().unwrap();
    
    if store.contains_key(&user.username) {
        return Ok(HttpResponse::Conflict().body("Username already exists"));
    }
    
    let hashed_password = hash(user.password.as_bytes(), DEFAULT_COST)
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to hash password"))?;
    
    store.insert(user.username.clone(), hashed_password);
    
    Ok(HttpResponse::Created().finish())
}

#[post("/login")]
pub async fn login(user: web::Json<User>) -> Result<HttpResponse, Error> {
    let store = USER_STORE.lock().unwrap();
    
    let hashed_password = match store.get(&user.username) {
        Some(pwd) => pwd,
        None => return Ok(HttpResponse::Unauthorized().body("Invalid username or password")),
    };
    
    if !verify(&user.password, hashed_password)
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to verify password"))? {
        return Ok(HttpResponse::Unauthorized().body("Invalid username or password"));
    }
    
    let token = uuid::Uuid::new_v4().to_string();
    let response = LoginResponse { token };
    
    Ok(HttpResponse::Ok().json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    fn cleanup_user(username: &str) {
        let mut store = USER_STORE.lock().unwrap();
        store.remove(username);
    }

    #[actix_web::test]
    async fn test_register() {
        let username = "testuser";
        cleanup_user(username);
        
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
        assert!(resp.status().is_success());
        
        cleanup_user(username);
    }

    #[actix_web::test]
    async fn test_login() {
        let username = "testuser";
        cleanup_user(username);
        
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
                password: "testpass".to_string(),
            })
            .to_request();
        let resp = test::call_service(&app, register_req).await;
        assert!(resp.status().is_success());

        // Now test login
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(&User {
                username: username.to_string(),
                password: "testpass".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        cleanup_user(username);
    }
} 