use axum::{
    Json,
    extract::{self, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
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
            let conn = database.pool.get().await.map_err(internal_error)?;

            repository::save_processed_payment(
                conn,
                &transaction.correlation_id.to_string(),
                transaction.requested_at,
                transaction.amount,
                service,
            )
            .await
            .map_err(internal_error)?;

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
