use crate::{
    db::MemoryDatabase,
    error_handling::internal_error,
    payment_processors::{self, service, structs::PaymentProcessorDTO},
    queue::RedisQueue,
    repository,
};
use axum::http::StatusCode;

pub async fn process_payment(
    memory_database: &MemoryDatabase,
    http_client: &reqwest::Client,
    process_queue: &RedisQueue,
    payload: PaymentProcessorDTO,
) -> Result<(), (StatusCode, String)> {
    let mut service = payment_processors::service::PaymentProcessorServices::Default;

    let response = match payment_processors::service::process_transaction(
        &http_client,
        &payload,
        payment_processors::service::PaymentProcessorServices::Default,
    )
    .await
    {
        Ok(res) => Ok(res),
        Err(_) => {
            service = payment_processors::service::PaymentProcessorServices::Fallback;
            service::process_transaction(
                &http_client,
                &payload,
                payment_processors::service::PaymentProcessorServices::Fallback,
            )
            .await
        }
    };

    match response {
        Ok(_res) => {

            repository::save_processed_payment(
                memory_database,
                payload.correlation_id,
                payload.requested_at,
                payload.amount,
                service,
            )
            .await
            .map_err(internal_error)?;
        }
        Err(_err) => {

            process_queue
                .push(payload.clone())
                .await
                .map_err(internal_error)?;
        }
    };

    Ok(())
}
