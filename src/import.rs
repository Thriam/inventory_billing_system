use crate::auth::verify_master_password;
use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ImportRequest {
    pub master_password: String,
    pub data: String, // The data to import, format can be JSON or CSV etc.
}

use crate::db::DB;
use serde_json::Value as JsonValue;
use surrealdb::sql::Value;

pub async fn import_data(req: web::Json<ImportRequest>) -> impl Responder {
    if !verify_master_password(&req.master_password) {
        return HttpResponse::Unauthorized().body("Invalid master password");
    }

    let db = DB.get().expect("DB not initialized");

    // Parse the data as JSON object with table names as keys and array of records as values
    let parsed_data: Result<JsonValue, _> = serde_json::from_str(&req.data);
    if parsed_data.is_err() {
        return HttpResponse::BadRequest().body("Invalid data format");
    }
    let parsed_data = parsed_data.unwrap();

    if !parsed_data.is_object() {
        return HttpResponse::BadRequest()
            .body("Data must be a JSON object with table names as keys");
    }

    let obj = parsed_data.as_object().unwrap();

    // Iterate over tables and insert/update records
    for (table, records) in obj.iter() {
        if !records.is_array() {
            continue; // skip if not array
        }
        let records_array = records.as_array().unwrap();

        for record in records_array {
            // Insert or update record in the table
            let query = format!("CREATE OR REPLACE {} CONTENT $content", table);
            let res = db.query(&query).bind(("content", record)).await;
            if res.is_err() {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to import data into table {}", table));
            }
        }
    }

    HttpResponse::Ok().body("Import completed successfully")
}
