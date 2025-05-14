use crate::auth::verify_master_password;
use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BackupRequest {
    pub master_password: String,
}

use crate::db::DB;
use serde_json::json;
use surrealdb::sql::Value;

pub async fn backup_data(req: web::Json<BackupRequest>) -> impl Responder {
    if !verify_master_password(&req.master_password) {
        return HttpResponse::Unauthorized().body("Invalid master password");
    }

    let db = DB.get().expect("DB not initialized");

    // Query all records from all tables (assuming tables: user, inventory, billing, ledger, etc.)
    // For demonstration, we will query all users and inventory items as example
    let mut backup_data = serde_json::Map::new();

    // Backup users
    let users_res = db.query("SELECT * FROM user").await;
    if let Ok(res) = users_res {
        if let Some(first) = res.get(0) {
            if let Ok(users) = first.result::<Vec<serde_json::Value>>() {
                backup_data.insert("user".to_string(), json!(users));
            }
        }
    }

    // Backup inventory items
    let inventory_res = db.query("SELECT * FROM inventory").await;
    if let Ok(res) = inventory_res {
        if let Some(first) = res.get(0) {
            if let Ok(items) = first.result::<Vec<serde_json::Value>>() {
                backup_data.insert("inventory".to_string(), json!(items));
            }
        }
    }

    // Add other tables similarly as needed

    let backup_json =
        serde_json::to_string_pretty(&backup_data).unwrap_or_else(|_| "{}".to_string());

    HttpResponse::Ok()
        .content_type("application/json")
        .body(backup_json)
}
