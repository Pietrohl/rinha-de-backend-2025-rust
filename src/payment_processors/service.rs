use std::f32::INFINITY;

use reqwest::StatusCode;

use crate::payment_processors::structs::{PaymentProcessorDTO, PaymentProcessorHealthCheckDTO};

const PAYMENT_PROCESSOR_DEFAULT_URL: &str = "http://localhost:8001";
const PAYMENT_PROCESSOR_FALLBACK_URL: &str = "http://localhost:8002";

#[derive(Debug, Clone)]
pub enum PaymentProcessorServices {
    Default,
    Fallback,
}

impl PaymentProcessorServices {
    pub fn get_url(&self) -> String {
        match self {
            PaymentProcessorServices::Default => std::env::var("PAYMENT_PROCESSOR_DEFAULT_URL")
                .unwrap_or(PAYMENT_PROCESSOR_DEFAULT_URL.to_string()),

            PaymentProcessorServices::Fallback => std::env::var("PAYMENT_PROCESSOR_FALLBACK_URL")
                .unwrap_or(PAYMENT_PROCESSOR_FALLBACK_URL.to_string()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            PaymentProcessorServices::Default => "default".to_string(),
            PaymentProcessorServices::Fallback => "fallback".to_string(),
        }
    }
}

impl Into<String> for PaymentProcessorServices {
    fn into(self) -> String {
        self.to_string()
    }
}

impl From<&str> for PaymentProcessorServices {
    fn from(s: &str) -> Self {
        match s {
            "default" => PaymentProcessorServices::Default,
            "fallback" => PaymentProcessorServices::Fallback,
            _ => PaymentProcessorServices::Default,
        }
    }
}

pub async fn process_transaction(
    client: &reqwest::Client,
    transaction: &PaymentProcessorDTO,
    service: PaymentProcessorServices,
) -> Result<(), (StatusCode, String)> {
    let response: Result<reqwest::Response, reqwest::Error> = client
        .post(format!("{}/payments", service.get_url()))
        .header("Content-Type", "application/json")
        .json(&transaction)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::OK {
                return Ok(());
            } else {
                let status = resp.status();
                return Err((status, "Failed to process transaction".to_string()));
            }
        }
        Err(_err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ));
        }
    }
}

static PAYMENT_PROCESSOR_HEALTH_FAILING: PaymentProcessorHealthCheckDTO =
    PaymentProcessorHealthCheckDTO {
        failing: true,
        min_response_time: INFINITY as i32,
    };

pub async fn get_service_health(
    client: &reqwest::Client,
    service: PaymentProcessorServices,
) -> PaymentProcessorHealthCheckDTO {
    let response: Result<reqwest::Response, reqwest::Error> = client
        .get(format!("{}/payments/service-health", service.get_url()))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::OK {
                let body = resp
                    .json()
                    .await
                    .unwrap_or(PAYMENT_PROCESSOR_HEALTH_FAILING);

                return body;
            } else {
                PAYMENT_PROCESSOR_HEALTH_FAILING
            }
        }
        Err(_err) => PAYMENT_PROCESSOR_HEALTH_FAILING,
    }
}
