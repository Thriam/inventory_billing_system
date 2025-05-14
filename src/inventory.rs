use crate::db::DB;
use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;

#[derive(Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub quantity: i32,
    pub price: f64,
}

#[derive(Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
    pub description: Option<String>,
    pub quantity: i32,
    pub price: f64,
}

async fn get_db() -> &'static Surreal<Client> {
    DB.get().expect("DB not initialized")
}

pub async fn create_item(req: web::Json<CreateItemRequest>) -> impl Responder {
    let db = get_db().await;
    let item = InventoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name.clone(),
        description: req.description.clone(),
        quantity: req.quantity,
        price: req.price,
    };
    if let Err(e) = db
        .create::<_, InventoryItem>("inventory")
        .content(&item)
        .await
    {
        return HttpResponse::InternalServerError().body(format!("Failed to create item: {}", e));
    }
    HttpResponse::Ok().json(item)
}

pub async fn list_items() -> impl Responder {
    let db = get_db().await;
    let query = "SELECT * FROM inventory";
    match db.query(query).await {
        Ok(res) => {
            let items = res
                .get(0)
                .and_then(|r| r.result::<Vec<InventoryItem>>().ok())
                .unwrap_or_default();
            HttpResponse::Ok().json(items)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to list items: {}", e)),
    }
}
