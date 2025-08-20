use std::{env, sync::Arc};

use crate::{
    db::MemoryDatabase,
    error_handling::internal_error,
    payment_processors::{
        self,
        structs::{PaymentProcessorDTO, PaymentProcessorHealth},
    },
    queue::RedisQueue,
    repository,
};
use axum::http::StatusCode;

pub fn select_service(
    payment_processors_health: &PaymentProcessorHealth,
) -> Option<payment_processors::service::PaymentProcessorServices> {
    let max_response_time = env::var("PAYMENT_PROCESSOR_MAX_RESPONSE_TIME")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(payment_processors::structs::PAYMENT_PROCESSOR_MAX_RESPONSE_TIME);

    if !payment_processors_health.default.failing
        || payment_processors_health.default.min_response_time < max_response_time
    {
        Some(payment_processors::service::PaymentProcessorServices::Default)
    } else if !payment_processors_health.fallback.failing
        || payment_processors_health.fallback.min_response_time < max_response_time
    {
        Some(payment_processors::service::PaymentProcessorServices::Fallback)
    } else {
        None
    }
}

pub async fn process_payment(
    memory_database: &MemoryDatabase,
    http_client: &reqwest::Client,
    process_queue: &RedisQueue,
    payment_processors_health: Arc<tokio::sync::RwLock<PaymentProcessorHealth>>,
    payload: PaymentProcessorDTO,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let health_guard = payment_processors_health.read().await;
    let service = select_service(&health_guard);

    if service.is_none() {
        process_queue.push(payload).await.map_err(internal_error)?;
        Ok((
            StatusCode::ACCEPTED,
            "Payment queued for processing".to_string(),
        ))
    } else {
        let service = service.unwrap();
        let response = payment_processors::service::process_transaction(
            http_client,
            &payload,
            &service,
        )
        .await;

        match response {
            Ok(_res) => {
                repository::save_processed_payment(
                    memory_database,
                    payload.correlation_id,
                    payload.requested_at,
                    payload.amount,
                    &service,
                )
                .await
                .map_err(internal_error)?;

                Ok((StatusCode::OK, "Payment processed successfully".to_string()))
            }
            Err(_err) => {
                process_queue.push(payload).await.map_err(internal_error)?;
                Ok((
                    StatusCode::ACCEPTED,
                    "Payment queued for processing".to_string(),
                ))
            }
        }
    }
}
