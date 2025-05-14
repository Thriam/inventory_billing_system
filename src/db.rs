use once_cell::sync::OnceCell;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;

pub static DB: OnceCell<Surreal<Client>> = OnceCell::new();
