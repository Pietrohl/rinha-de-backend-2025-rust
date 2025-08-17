use std::time::Duration;

use crate::{
    db::{DEFAULT_DATABASE_URL, DEFAULT_MEMORY_DATABASE_URL, MemoryDatabaseConnection},
    error_handling::internal_error,
    structs::AppState,
};
use axum::{error_handling::HandleErrorLayer, extract::Request, handler};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use bb8_redis::RedisConnectionManager;
use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server,
};
use tokio_postgres::NoTls;
use tower::{Service, ServiceBuilder, buffer, limit::ConcurrencyLimitLayer};
// use crate::payment_processors;
mod controller;
mod db;
mod error_handling;
pub mod payment_processors;
mod repository;
mod structs;

#[tokio::main]
async fn main() {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(2))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(500)
        .build()
        .unwrap();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let memory_database_url = std::env::var("MEMORY_DATABASE_URL")
        .unwrap_or_else(|_| DEFAULT_MEMORY_DATABASE_URL.to_string());

    let manager = PostgresConnectionManager::new_from_stringlike(database_url, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
        bb8::Pool::builder().build(manager).await.unwrap();

    let database = db::PostgresDatabase::new(pool);

    let memory_manager = RedisConnectionManager::new(memory_database_url).unwrap();
    let memory_pool: MemoryDatabaseConnection =
        bb8::Pool::builder().build(memory_manager).await.unwrap();

    let memory_database = db::MemoryDatabase::new(memory_pool);

    let app_state = AppState {
        database,
        memory_database,
        http_client,
    };

    let priority_route = axum::Router::new()
        .route("/payments", axum::routing::post(controller::payments))
        .layer(ConcurrencyLimitLayer::new(1024));
        
        
       let app  = axum::Router::new()
        .route(
            "/payments-summary",
            axum::routing::get(controller::payments_summary),
        )
        .route(
            "/purge-payments",
            axum::routing::post(controller::purge_payments),
        )
        .layer(ConcurrencyLimitLayer::new(32))
        .merge(priority_route)
        .with_state(app_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "9999".to_string());

    let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", &port))
        .expect(&format!("error listening to socket 0.0.0.0:{}", &port));
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::TcpListener::from_std(listener).expect("error parsing std listener");

    eprintln!("Server up!");

    axum::serve(listener, app).await.unwrap();
}
