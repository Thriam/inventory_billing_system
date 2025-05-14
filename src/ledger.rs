use crate::db::DB;
use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;

#[derive(Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: String,
    pub description: String,
    pub amount: f64,
    pub date: String,
    pub entry_type: String, // e.g., "debit" or "credit"
}

#[derive(Deserialize)]
pub struct CreateLedgerEntryRequest {
    pub description: String,
    pub amount: f64,
    pub date: String,
    pub entry_type: String,
}

async fn get_db() -> &'static Surreal<Client> {
    DB.get().expect("DB not initialized")
}

pub async fn create_ledger_entry(req: web::Json<CreateLedgerEntryRequest>) -> impl Responder {
    let db = get_db().await;
    let entry = LedgerEntry {
        id: uuid::Uuid::new_v4().to_string(),
        description: req.description.clone(),
        amount: req.amount,
        date: req.date.clone(),
        entry_type: req.entry_type.clone(),
    };
    if let Err(e) = db.create::<_, LedgerEntry>("ledger").content(&entry).await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to create ledger entry: {}", e));
    }
    HttpResponse::Ok().json(entry)
}

pub async fn list_ledger_entries() -> impl Responder {
    let db = get_db().await;
    let query = "SELECT * FROM ledger";
    match db.query(query).await {
        Ok(res) => {
            let entries = res
                .get(0)
                .and_then(|r| r.result::<Vec<LedgerEntry>>().ok())
                .unwrap_or_default();
            HttpResponse::Ok().json(entries)
        }
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Failed to list ledger entries: {}", e)),
    }
}
