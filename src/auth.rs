use crate::db::DB;
use crate::mail::send_email;
use actix_web::{HttpResponse, Responder, web};
use bcrypt::{DEFAULT_COST, hash, verify};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;
use tokio::sync::RwLock;
use uuid::Uuid;

static OTP_STORE: OnceCell<RwLock<HashMap<String, String>>> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct PasswordChangeRequest {
    pub username: String,
    pub old_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct PasswordResetRequest {
    pub username: String,
    pub otp: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct OTPRequest {
    pub username: String,
}

async fn get_db() -> &'static Surreal<Client> {
    DB.get().expect("DB not initialized")
}

async fn get_user_by_username(db: &Surreal<Client>, username: &str) -> Option<User> {
    let query = "SELECT * FROM user WHERE username = $username LIMIT 1";
    let res = db.query(query).bind(("username", username)).await.ok()?;
    let result = res.get(0)?.result::<Vec<User>>().ok()?;
    result.into_iter().next()
}

async fn create_user(db: &Surreal<Client>, user: &User) -> Result<(), surrealdb::Error> {
    db.create::<_, User>("user").content(user).await?;
    Ok(())
}

pub async fn register_user(req: web::Json<RegisterRequest>) -> impl Responder {
    let db = get_db().await;
    if get_user_by_username(db, &req.username).await.is_some() {
        return HttpResponse::BadRequest().body("Username already exists");
    }
    let password_hash = hash(&req.password, DEFAULT_COST).unwrap();
    let user = User {
        id: Uuid::new_v4().to_string(),
        username: req.username.clone(),
        email: req.email.clone(),
        password_hash,
        is_admin: false,
    };
    if let Err(e) = create_user(db, &user).await {
        return HttpResponse::InternalServerError().body(format!("Failed to create user: {}", e));
    }
    HttpResponse::Ok().body("User registered successfully")
}

pub async fn login_user(req: web::Json<LoginRequest>) -> impl Responder {
    let db = get_db().await;
    if let Some(user) = get_user_by_username(db, &req.username).await {
        if verify(&req.password, &user.password_hash).unwrap_or(false) {
            return HttpResponse::Ok().body("Login successful");
        }
    }
    HttpResponse::Unauthorized().body("Invalid username or password")
}

pub async fn change_password(req: web::Json<PasswordChangeRequest>) -> impl Responder {
    let db = get_db().await;
    if let Some(mut user) = get_user_by_username(db, &req.username).await {
        if verify(&req.old_password, &user.password_hash).unwrap_or(false) {
            let new_hash = hash(&req.new_password, DEFAULT_COST).unwrap();
            user.password_hash = new_hash.clone();
            let update_query =
                "UPDATE user SET password_hash = $password_hash WHERE username = $username";
            if let Err(e) = db
                .query(update_query)
                .bind(("password_hash", new_hash))
                .bind(("username", &req.username))
                .await
            {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to update password: {}", e));
            }
            // Send OTP email for password change confirmation
            let _ = send_email(
                user.email.clone(),
                "Your password has been changed.".to_string(),
            )
            .await;
            return HttpResponse::Ok().body("Password changed successfully");
        }
    }
    HttpResponse::Unauthorized().body("Invalid username or password")
}

pub async fn request_password_reset(req: web::Json<OTPRequest>) -> impl Responder {
    let db = get_db().await;
    if let Some(user) = get_user_by_username(db, &req.username).await {
        let otp = format!("{:06}", rand::random::<u32>() % 1_000_000);
        OTP_STORE.get_or_init(|| RwLock::new(HashMap::new()));
        let otp_store = OTP_STORE.get().unwrap();
        {
            let mut store = otp_store.write().await;
            store.insert(req.username.clone(), otp.clone());
        }
        let _ = send_email(user.email.clone(), otp.clone()).await;
        return HttpResponse::Ok().body("OTP sent to registered email");
    }
    HttpResponse::BadRequest().body("User not found")
}

pub async fn reset_password(req: web::Json<PasswordResetRequest>) -> impl Responder {
    OTP_STORE.get_or_init(|| RwLock::new(HashMap::new()));
    let otp_store = OTP_STORE.get().unwrap();
    {
        let store = otp_store.read().await;
        if let Some(stored_otp) = store.get(&req.username) {
            if stored_otp != &req.otp {
                return HttpResponse::Unauthorized().body("Invalid OTP");
            }
        } else {
            return HttpResponse::Unauthorized().body("OTP not found");
        }
    }
    let db = get_db().await;
    if let Some(mut user) = get_user_by_username(db, &req.username).await {
        let new_hash = hash(&req.new_password, DEFAULT_COST).unwrap();
        user.password_hash = new_hash.clone();
        let update_query =
            "UPDATE user SET password_hash = $password_hash WHERE username = $username";
        if let Err(e) = db
            .query(update_query)
            .bind(("password_hash", new_hash))
            .bind(("username", &req.username))
            .await
        {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to update password: {}", e));
        }
        return HttpResponse::Ok().body("Password reset successfully");
    }
    HttpResponse::BadRequest().body("User not found")
}

// Admin account with fixed credentials
pub fn get_admin_account() -> User {
    User {
        id: "admin".to_string(),
        username: "thriamindustries".to_string(),
        email: "thriamindustries@gmail.com".to_string(),
        password_hash: hash("123", DEFAULT_COST).unwrap(),
        is_admin: true,
    }
}

// Verify master password for backup/import access
pub fn verify_master_password(password: &str) -> bool {
    let admin = get_admin_account();
    verify(password, &admin.password_hash).unwrap_or(false)
}

pub async fn logout_user() -> impl Responder {
    // Since this is stateless auth, logout can be handled on client side by clearing tokens
    HttpResponse::Ok().body("Logout successful")
}
