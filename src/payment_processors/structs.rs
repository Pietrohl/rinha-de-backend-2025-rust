use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Deserialize, Copy)]
pub struct PaymentProcessorHealthCheckDTO {
    pub failing: bool,
    #[serde(rename = "minResponseTime")]
    pub min_response_time: i32,
}




#[derive(Debug, Clone)]
pub struct PaymentProcessorHealth {
    pub default: PaymentProcessorHealthCheckDTO,
    pub fallback: PaymentProcessorHealthCheckDTO,
}



pub const PAYMENT_PROCESSOR_MAX_RESPONSE_TIME: i32 = 200; // Maximum response time in milliseconds for a payment processor to be considered healthy 