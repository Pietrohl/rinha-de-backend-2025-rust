use std::sync::Arc;

use axum::{
    Json,
    extract::{self, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::{
    error_handling::internal_error,
    repository,
    structs::{AppState, PaymentDTO, PaymentSummaryQuery},
};
use crate::{payment_processors, service::process_payment};

pub async fn payments(
    State(state): State<Arc<AppState>>,
    extract::Json(payload): extract::Json<PaymentDTO>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let transaction: payment_processors::structs::PaymentProcessorDTO = payload.into();

    process_payment(
        &state.memory_database,
        &state.http_client,
        &state.redis_queue,
        state.processor_health.clone(),
        transaction,
    )
    .await
}

pub async fn payments_summary(
    State(state): State<Arc<AppState>>,

    extract::Query(query_params): extract::Query<PaymentSummaryQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let summary = repository::get_payments_summary(
        &state.memory_database,
        &state.database,
        query_params.from,
        query_params.to,
    )
    .await
    .map_err(|e| internal_error(&*e))?;

    Ok((StatusCode::OK, Json(summary)))
}

pub async fn purge_payments(
    State(state): State<Arc<AppState>>,
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
