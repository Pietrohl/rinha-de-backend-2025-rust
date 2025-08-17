use axum::extract::Request;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use hyper::body::Incoming;
use tokio_postgres::NoTls;
use hyper_util::{rt::{TokioExecutor, TokioIo}, server};
use tower::Service;
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
    let pool: Pool<PostgresConnectionManager<NoTls>> = bb8::Pool::builder()
        .min_idle(50)
        .retry_connection(true)
        .max_size(100)
        .build(manager)
        .await
        .unwrap();

    let database = db::PostgresDatabase::new(pool.clone());

    let app = axum::Router::new()
        .route("/payments", axum::routing::post(controller::payments))
        .route(
            "/payments-summary",
            axum::routing::get(controller::payments_summary),
        )
        .route(
            "/purge-payments",
            axum::routing::post(controller::purge_payments),
        )
        .with_state(database);

    let port = std::env::var("PORT").unwrap_or_else(|_| "9999".to_string());

    let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", &port))
        .expect(&format!("error listening to socket 0.0.0.0:{}", &port));
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::TcpListener::from_std(listener).expect("error parsing std listener");

  
    eprintln!("Server up!");

    loop {
        let (stream, _) = listener.accept().await.unwrap();

        let tower_service = app.clone();

        tokio::spawn(async move {
            let socket = TokioIo::new(stream);

            let service = hyper::service::service_fn(move |request: Request<Incoming>| {
                tower_service.clone().call(request)
            });

            if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection(socket, service)
                .await
            {
                eprintln!("failed to serve connection: {err:#}");
            }
        });
    }




}
