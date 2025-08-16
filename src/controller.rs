use axum::{
    extract::{self, Path, State},
    http::StatusCode,
    response::IntoResponse, Json,
};

use crate::{
    db::PostgresDatabase,
    structs::{PaymentDTO, PaymentSummaryQuery, PaymentsServiceSummary, PaymentsSummaryResponseDTO},
};

pub async fn payments(
    extract::Json(payload): extract::Json<PaymentDTO>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Here you would typically handle the payment logic

    Ok((
        StatusCode::OK,
        format!(
            "Payment processed successfully payload.correlation_id: {}",
            payload.correlation_id
        ),
    ))
}

pub async fn payments_summary(
    extract::Query(query_params): extract::Query<PaymentSummaryQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let summary = PaymentsSummaryResponseDTO {
        default: PaymentsServiceSummary {
            total_requests: 100,  // Example value
            total_amount: 1000.0, // Example value
        },
        fallback: PaymentsServiceSummary {
            total_requests: 50,  // Example value
            total_amount: 500.0, // Example value
        },
    };
    Ok((StatusCode::OK, Json(summary)))
}
