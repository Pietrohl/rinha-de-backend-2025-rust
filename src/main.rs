use std::{env, sync::Arc, time::Duration};

use crate::{
    db::{DEFAULT_DATABASE_URL, DEFAULT_MEMORY_DATABASE_URL, MemoryDatabaseConnection},
    structs::AppState,
};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use bb8_redis::RedisConnectionManager;
use tokio_postgres::NoTls;
use tower::limit::ConcurrencyLimitLayer;
// use crate::payment_processors;
mod controller;
mod db;
mod error_handling;
pub mod payment_processors;
mod queue;
mod repository;
mod service;
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
    let manager = PostgresConnectionManager::new_from_stringlike(database_url, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
        bb8::Pool::builder().build(manager).await.unwrap();
    let database = db::PostgresDatabase::new(pool);

    let memory_database_url = std::env::var("MEMORY_DATABASE_URL")
        .unwrap_or_else(|_| DEFAULT_MEMORY_DATABASE_URL.to_string());
    let memory_manager = RedisConnectionManager::new(memory_database_url).unwrap();
    let memory_pool: MemoryDatabaseConnection = bb8::Pool::builder()
        .min_idle(10)
        .max_size(32)
        .build(memory_manager)
        .await
        .unwrap();

    let memory_database = db::MemoryDatabase::new(memory_pool.clone());
    let redis_queue = queue::RedisQueue::new(memory_pool);

    let app_state = Arc::new(AppState {
        database,
        memory_database,
        http_client,
        redis_queue,
        processor_health: payment_processors::structs::PaymentProcessorHealth {
            default: payment_processors::structs::PaymentProcessorHealthCheckDTO {
                failing: false,
                min_response_time: 0,
            },
            fallback: payment_processors::structs::PaymentProcessorHealthCheckDTO {
                failing: false,
                min_response_time: 0,
            },
        },
    });

    let mut workers = Vec::new();
    let num_workers = env::var("NUM_WORKERS")
        .unwrap_or_else(|_| "50".to_string())       
        .parse::<usize>().unwrap_or(50);

    //  inseatd of one worker thread, span 100 worker threads
    // each worker thread will check the redis queue and process payments
    for _ in 0..num_workers {
        let worker_state = app_state.clone();

        let tread = tokio::spawn(async move {
            //Check redis queue in 100ms intervals, if there are any payments, process them
            let redis_queue = &worker_state.redis_queue;
            let memory_database = &worker_state.memory_database;
            let client = &worker_state.http_client;
            loop {
                if let Ok(Some(payment)) = redis_queue.pop().await {
                    let mut retries = 0;
                    loop {
                        match service::process_payment(
                            &memory_database,
                            &client,
                            &redis_queue,
                            payment.clone(),
                        )
                        .await
                        {
                            Ok(_) => break,
                            Err(e) => {
                                retries += 1;
                                if retries >= 100 {
                                    eprintln!(
                                        "Failed to process payment after 100 retries: {:?}",
                                        e
                                    );
                                    break;
                                }
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        workers.push(tread);
    }

    let priority_route = axum::Router::new()
        .route("/payments", axum::routing::post(controller::payments))
        .layer(ConcurrencyLimitLayer::new(1024));

    let app = axum::Router::new()
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
        .with_state(app_state.clone());

    let port = std::env::var("PORT").unwrap_or_else(|_| "9999".to_string());

    let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", &port))
        .expect(&format!("error listening to socket 0.0.0.0:{}", &port));
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::TcpListener::from_std(listener).expect("error parsing std listener");

    eprintln!("Server up!");

    axum::serve(listener, app).await.unwrap();

    // Wait for all worker threads to finish
    for tread in workers {
        tread.await.unwrap();
    }
    eprintln!("Server down!");
}
