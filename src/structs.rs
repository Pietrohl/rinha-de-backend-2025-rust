use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::payment_processors;

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentDTO {
    #[serde(rename = "correlationId")]
    pub correlation_id: Uuid,
    pub amount: f64,
}


#[derive(Debug, Clone)]
pub struct PaymentDatabaseEntry {
    pub correlation_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub amount: f64,
    pub service: payment_processors::service::PaymentProcessorServices,
}


#[derive(Debug, Clone, Deserialize)]
pub struct PaymentSummaryQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Debug)]
pub struct PaymentsServiceSummary {
    pub total_requests: u32,
    pub total_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentsSummaryResponseDTO {
    pub default: PaymentsServiceSummary,
    pub fallback: PaymentsServiceSummary,
}

impl From<PaymentDTO> for payment_processors::structs::PaymentProcessorDTO {
    fn from(val: PaymentDTO) -> Self {
        payment_processors::structs::PaymentProcessorDTO {
            correlation_id: val.correlation_id,
            amount: val.amount , // Convert to cents
            requested_at: Utc::now(),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub database: crate::db::PostgresDatabase,
    pub memory_database: crate::db::MemoryDatabase,
    pub http_client: reqwest::Client,
    pub redis_queue: crate::queue::RedisQueue,
    pub processor_health: Arc<RwLock<payment_processors::structs::PaymentProcessorHealth>>,
    pub health_check_channel: crate::pubsub::HealthCheckChannel,
}