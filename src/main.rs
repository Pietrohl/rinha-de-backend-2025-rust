use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

use crate::db::DATABASE_URL;
// use crate::payment_processors;
mod controller;
mod db;
mod structs;
pub mod payment_processors;

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

    let listener = std::net::TcpListener::bind("0.0.0.0:5000")
        .expect("error listening to socket 0.0.0.0:3000");
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::TcpListener::from_std(listener).expect("error parsing std listener");

    axum::serve(listener, app).await.unwrap();

    eprintln!("Server up!");
}
