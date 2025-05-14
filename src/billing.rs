use crate::db::DB;
use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;

#[derive(Serialize, Deserialize)]
pub struct Bill {
    pub id: String,
    pub items: Vec<BillItem>,
    pub total_amount: f64,
    pub customer_name: String,
    pub date: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BillItem {
    pub item_id: String,
    pub quantity: i32,
    pub price: f64,
}

#[derive(Deserialize)]
pub struct CreateBillRequest {
    pub items: Vec<BillItem>,
    pub customer_name: String,
    pub date: String,
}

async fn get_db() -> &'static Surreal<Client> {
    DB.get().expect("DB not initialized")
}

pub async fn create_bill(req: web::Json<CreateBillRequest>) -> impl Responder {
    let db = get_db().await;
    let total_amount = req
        .items
        .iter()
        .map(|item| item.price * item.quantity as f64)
        .sum();
    let bill = Bill {
        id: uuid::Uuid::new_v4().to_string(),
        items: req.items.clone(),
        total_amount,
        customer_name: req.customer_name.clone(),
        date: req.date.clone(),
    };
    if let Err(e) = db.create::<_, Bill>("bill").content(&bill).await {
        return HttpResponse::InternalServerError().body(format!("Failed to create bill: {}", e));
    }
    HttpResponse::Ok().json(bill)
}

pub async fn list_bills() -> impl Responder {
    let db = get_db().await;
    let query = "SELECT * FROM bill";
    match db.query(query).await {
        Ok(res) => {
            let bills = res
                .get(0)
                .and_then(|r| r.result::<Vec<Bill>>().ok())
                .unwrap_or_default();
            HttpResponse::Ok().json(bills)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to list bills: {}", e)),
    }
}
