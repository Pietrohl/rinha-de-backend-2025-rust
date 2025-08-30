use std::{env, sync::Arc, time::Duration};
use tokio::{join, sync::RwLock};

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
mod pubsub;
mod repository;
mod service;
mod structs;

#[tokio::main]
async fn main() {
    println!("Starting the payment processing server...");

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(2))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(500)
        .build()
        .unwrap();


    // Constants
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let memory_database_url = std::env::var("MEMORY_DATABASE_URL")
        .unwrap_or_else(|_| DEFAULT_MEMORY_DATABASE_URL.to_string());
    let num_workers = env::var("NUM_WORKERS")
        .unwrap_or_else(|_| "50".to_string())
        .parse::<usize>()
        .unwrap_or(50);
    let instance = std::env::var("INSTANCE").unwrap_or_else(|_| "".to_string());
        


    println!("Starting Postgres Connection Pool")
    let manager = PostgresConnectionManager::new_from_stringlike(database_url, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
    bb8::Pool::builder().build(manager).await.unwrap();
    let database = db::PostgresDatabase::new(pool);

    
    println!("Starting Redis Connection Pool")
    let memory_manager = RedisConnectionManager::new(memory_database_url).unwrap();     
    let memory_pool: MemoryDatabaseConnection = bb8::Pool::builder()
        .min_idle(10)
        .max_size(32)
        .build(memory_manager)
        .await
        .unwrap();
    let memory_database = db::MemoryDatabase::new(memory_pool.clone());
    
    println!("Starting Channel")
    let health_check_channel = pubsub::HealthCheckChannel::new(memory_pool.clone());

    println!("Starting DLQ")
    let redis_queue = queue::RedisQueue::new(memory_pool);


    

    

    println!("Creating App State...");

    let processor_health = Arc::new(RwLock::new(
        payment_processors::structs::PaymentProcessorHealth {
            default: payment_processors::structs::PaymentProcessorHealthCheckDTO {
                failing: false,
                min_response_time: 0,
            },
            fallback: payment_processors::structs::PaymentProcessorHealthCheckDTO {
                failing: false,
                min_response_time: 0,
            },
        },
    ));

    
    let health_check_http_client = http_client.clone();
    let processor_health_clone = processor_health.clone();

    let app_state = Arc::new(AppState {
        database,
        memory_database,
        http_client,
        redis_queue,
        processor_health,
        health_check_channel,
    });


    println!("App state Created!");




    let app_state_clone = app_state.clone();
    if instance == "MASTER" {
        println!("Starting health check thread");
        tokio::spawn(async move {
            loop {
                


                let (default, fallback) = join!(
                    payment_processors::service::get_service_health(
                        &health_check_http_client,
                        payment_processors::service::PaymentProcessorServices::Default
                    ),
                    payment_processors::service::get_service_health(
                        &health_check_http_client,
                        payment_processors::service::PaymentProcessorServices::Fallback
                    )
                );

                // Example: let health = payment_processors::service::check_health().await;
                let health =
                payment_processors::structs::PaymentProcessorHealth { default, fallback };

                let channel = &app_state_clone.health_check_channel.update(health.clone());

                let mut guard = processor_health_clone.write().await;
                *guard = health;

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    } else {
        
        println!("Starting health check thread");
        tokio::spawn(async move {
            loop {
                let subscriber = &app_state_clone.health_check_channel.subcribe();

                let msg = subscriber.get_message().await;


                match msg {
                    Ok(message) => {
                        let health: payment_processors::structs::PaymentProcessorHealth = message.get_payload().unwrap();
                        {
                            let mut guard = processor_health_clone.write().await;
                            *guard = health;
                        }   
                    }
                    Err(e) => {
                        eprintln!("Subscriber error: {:?}", e);
                        break;
                    }
                }

            }
        });
    }
        
        
        

    
    
    
    println!("Starting worker threads")
    let mut workers = Vec::new();
    for _ in 0..num_workers {
        let worker_state = app_state.clone();

        let tread = tokio::spawn(async move {
            let redis_queue = &worker_state.redis_queue;
            let memory_database = &worker_state.memory_database;
            let client = &worker_state.http_client;
            loop {
                let health_guard = worker_state.processor_health.read().await;
                if let Some(_service) = service::select_service(
                    &health_guard,
                ) {
                    // If a service is available, process payments
                    if let Ok(Some(payment)) = redis_queue.pop().await {
                        let mut retries = 0;
                        loop {
                            match service::process_payment(
                                memory_database,
                                client,
                                redis_queue,
                                worker_state.processor_health.clone(),
                                payment,
                            )
                            .await
                            {
                                Ok(_) => break,
                                Err(e) => {
                                    retries += 1;
                                    if retries >= 100 {
                                        eprintln!(
                                            "Failed to process payment after 100 retries: {e:?}"
                                        );
                                        break;
                                    }
                                    tokio::time::sleep(Duration::from_millis(50)).await;
                                }
                            }
                        }
                    }
                } 
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        workers.push(tread);
    }



    println!("Starting server")
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


    let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", &port))
        .unwrap_or_else(|_| panic!("error listening to socket 0.0.0.0:{}", &port));
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::TcpListener::from_std(listener).expect("error parsing std listener");

    eprintln!("Server up!");

    axum::serve(listener, app).await.unwrap();

    // Wait for all worker threads to finish
    if let Some(tread) = workers.into_iter().next() {
        tread.await.unwrap();
    }
    eprintln!("Server down!");
}
