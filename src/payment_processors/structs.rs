use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct PaymentProcessorDTO {
    pub corrlation_id: String,
    pub amount: i32,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentProcessorResponseDTO {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentProcessorHealthCheckDTO {
    pub failing: String,
    pub min_response_time: i32,
}
