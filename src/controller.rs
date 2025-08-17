use axum::{
    Json,
    extract::{self, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::{
    error_handling::internal_error,
    payment_processors::{self, service},
    repository,
    structs::{AppState, PaymentDTO, PaymentSummaryQuery},
};

pub async fn payments(
    State(state): State<AppState>,
    extract::Json(payload): extract::Json<PaymentDTO>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let transaction: payment_processors::structs::PaymentProcessorDTO = payload.into();
    let mut service = payment_processors::service::PaymentProcessorServices::Default;


    let response = match payment_processors::service::process_transaction(
        &state.http_client,
        &transaction,
        payment_processors::service::PaymentProcessorServices::Default,
    )
    .await
    {
        Ok(res) => Ok(res),
        Err(_) => {
            service = payment_processors::service::PaymentProcessorServices::Fallback;
            service::process_transaction(
               &state.http_client,
                &transaction,
                payment_processors::service::PaymentProcessorServices::Fallback,
            )
            .await
        }
    };

    match response {
        Ok(_res) => {
            let memory_database = &state.memory_database;

            repository::save_processed_payment(
                memory_database,
                transaction.correlation_id,
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
    State(state): State<AppState>,

    extract::Query(query_params): extract::Query<PaymentSummaryQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    

    let summary = repository::get_payments_summary(&state.memory_database, &state.database, query_params.from, query_params.to)
        .await
        .map_err(|e| internal_error(&*e))?;

    Ok((StatusCode::OK, Json(summary)))
}

pub async fn purge_payments(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conn = state.database.pool.get().await.map_err(internal_error)?;

    let rows_affected = repository::purge_payments(conn)
        .await
        .map_err(|e| internal_error(&*e))?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": format!("Purged {} payments", rows_affected) })),
    ))
}
