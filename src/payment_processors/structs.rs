use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct PaymentProcessorDTO {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
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
