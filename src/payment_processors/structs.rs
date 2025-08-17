use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct PaymentProcessorDTO {
    #[serde(rename = "correlationId")]
    pub correlation_id: Uuid,
    pub amount: f64,
    #[serde(rename = "requestedAt")]
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentProcessorResponseDTO {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentProcessorHealthCheckDTO {
    pub failing: String,
    #[serde(rename = "minResponseTime")]
    pub min_response_time: i32,
}
