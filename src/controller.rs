use axum::{
    Json,
    extract::{self, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::{
    db::PostgresDatabase,
    error_handling::internal_error,
    payment_processors::{self, service},
    repository,
    structs::{PaymentDTO, PaymentSummaryQuery},
};

pub async fn payments(
    State(database): State<PostgresDatabase>,
    extract::Json(payload): extract::Json<PaymentDTO>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let transaction: payment_processors::structs::PaymentProcessorDTO = payload.into();
    let mut service = payment_processors::service::PaymentProcessorServices::Default;

    let response = match payment_processors::service::process_transaction(
        &transaction,
        payment_processors::service::PaymentProcessorServices::Default,
    )
    .await
    {
        Ok(res) => Ok(res),
        Err(_) => {
            service = payment_processors::service::PaymentProcessorServices::Fallback;
            service::process_transaction(
                &transaction,
                payment_processors::service::PaymentProcessorServices::Fallback,
            )
            .await
        }
    };

    match response {
        Ok(_res) => {
            println!("Payment processed successfully: {:?}", json!(&transaction));
            println!("Service used: {:?}", service.to_string());
            println!("Pulling connection from the database...");
            let conn = database.pool.get().await.map_err(internal_error)?;
            println!("Saving processed payment to the database...");
            repository::save_processed_payment(
                conn,
                transaction.correlation_id,
                transaction.requested_at,
                transaction.amount,
                service,
            )
            .await
            .map_err(internal_error)?;

            println!("Payment saved to the database successfully.");

            Ok((
                StatusCode::OK,
                format!(
                    "Payment processed successfully payload.correlation_id: {}",
                    transaction.correlation_id
                ),
            ))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Payment processing failed: {}, request: {} ",
                e.1,
                json!(&transaction)
            ),
        )),
    }
}

pub async fn payments_summary(
    State(database): State<PostgresDatabase>,
    extract::Query(query_params): extract::Query<PaymentSummaryQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = database.pool.get().await.map_err(internal_error)?;

    let summary = repository::get_payments_summary(conn, query_params.from, query_params.to)
        .await
        .map_err(internal_error)?;

    Ok((StatusCode::OK, Json(summary)))
}


pub async fn purge_payments(
    State(database): State<PostgresDatabase>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = database.pool.get().await.map_err(internal_error)?;

    let rows_affected = repository::purge_payments(conn)
        .await
        .map_err(internal_error)?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": format!("Purged {} payments", rows_affected) })),
    ))
}