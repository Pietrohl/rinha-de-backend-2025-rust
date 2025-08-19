use std::sync::Arc;

use crate::{
    db::MemoryDatabase,
    error_handling::internal_error,
    payment_processors::{
        self, service,
        structs::{PaymentProcessorDTO, PaymentProcessorHealth},
    },
    queue::RedisQueue,
    repository,
};
use axum::http::StatusCode;

fn select_service(
    payment_processors_health: &PaymentProcessorHealth,
) -> Option<payment_processors::service::PaymentProcessorServices> {
    if !payment_processors_health.default.failing
        || payment_processors_health.default.min_response_time
            < payment_processors::structs::PAYMENT_PROCESSOR_MAX_RESPONSE_TIME
    {
        Some(payment_processors::service::PaymentProcessorServices::Default)
    } else if !payment_processors_health.fallback.failing
        || payment_processors_health.fallback.min_response_time
            < payment_processors::structs::PAYMENT_PROCESSOR_MAX_RESPONSE_TIME
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
    let service = select_service(&*health_guard);

    if service.is_none() {
        process_queue
            .push(payload.clone())
            .await
            .map_err(internal_error)?;
        return Ok((
            StatusCode::ACCEPTED,
            "Payment queued for processing".to_string(),
        ));
    } else {
        let response = payment_processors::service::process_transaction(
            &http_client,
            &payload,
            payment_processors::service::PaymentProcessorServices::Default,
        )
        .await;

        match response {
            Ok(_res) => {
                repository::save_processed_payment(
                    memory_database,
                    payload.correlation_id,
                    payload.requested_at,
                    payload.amount,
                    service.unwrap(),
                )
                .await
                .map_err(internal_error)?;

                Ok((StatusCode::OK, "Payment processed successfully".to_string()))
            }
            Err(_err) => {
                process_queue
                    .push(payload.clone())
                    .await
                    .map_err(internal_error)?;
                return Ok((
                    StatusCode::ACCEPTED,
                    "Payment queued for processing".to_string(),
                ));
            }
        }
    }
}
