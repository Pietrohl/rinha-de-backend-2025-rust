use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

use crate::db::DATABASE_URL;
// use crate::payment_processors;
mod controller;
mod db;
mod error_handling;
pub mod payment_processors;
mod repository;
mod structs;

#[tokio::main]
async fn main() {
    let manager = PostgresConnectionManager::new_from_stringlike(DATABASE_URL, NoTls).unwrap();
    let pool = Pool::builder().build(manager).await.unwrap();
    let database = db::PostgresDatabase::new(pool.clone());

    let app = axum::Router::new()
        .route("/payments", axum::routing::post(controller::payments))
        .route(
            "/payments-summary",
            axum::routing::get(controller::payments_summary),
        )
        .with_state(database);

    let port = std::env::var("PORT").unwrap_or_else(|_| "9999".to_string());

    let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", &port))
        .expect(&format!("error listening to socket 0.0.0.0:{}", &port));
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::TcpListener::from_std(listener).expect("error parsing std listener");

    axum::serve(listener, app).await.unwrap();

    eprintln!("Server up!");
}
