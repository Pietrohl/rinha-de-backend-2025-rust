use crate::structs::PaymentsServiceSummary;
use tokio_postgres::Row;

use chrono::{DateTime, Utc};
use tokio_postgres::Error;

use crate::{db::BB8PooledConnection, payment_processors, structs::PaymentsSummaryResponseDTO};

const INSERT_QUERY: &str = "INSERT INTO processed_payments (correlation_id, processed_at, amount, service) VALUES ($1, $2, $3, $4)";

const SUMMARY_QUERY: &str = "SELECT service, COUNT(*) as total_requests, COALESCE(SUM(amount), 0) as total_amount FROM processed_payments WHERE ($1::timestamptz IS NULL OR processed_at >= $1) AND ($2::timestamptz IS NULL OR processed_at <= $2) GROUP BY service";

fn extract_summary(rows: &[Row], service: &str) -> PaymentsServiceSummary {
    for row in rows {
        let row_service: String = row.get("service");
        if row_service == service {
            let total_requests: i64 = row.get("total_requests");
            let mut total_amount: f64 = row.get("total_amount");
            total_amount = total_amount / 100.00; // Convert cents to dollars
            return PaymentsServiceSummary {
                total_requests: total_requests as u32,
                total_amount,
            };
        }
    }
    PaymentsServiceSummary {
        total_requests: 0,
        total_amount: 0.0,
    }
}

pub async fn save_processed_payment<'a>(
    conn: BB8PooledConnection<'a>,
    correlation_id: &str,
    date: DateTime<Utc>,
    amount: f64,
    service: payment_processors::service::PaymentProcessorServices,
) -> Result<(), Error> {
    conn.query(
        INSERT_QUERY,
        &[
            &correlation_id,
            &date.to_rfc3339(),
            &((amount * 100.0).round() as i64),
            &service.to_string(),
        ],
    )
    .await?;
    Ok(())
}

pub async fn get_payments_summary<'a>(
    conn: BB8PooledConnection<'a>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<PaymentsSummaryResponseDTO, Error> {
    let from_param = from.map(|dt| dt.to_rfc3339());
    let to_param = to.map(|dt| dt.to_rfc3339());

    let rows = conn
        .query(
            SUMMARY_QUERY,
            &[&from_param.as_deref(), &to_param.as_deref()],
        )
        .await?;

    let summary = PaymentsSummaryResponseDTO {
        default: extract_summary(&rows, "default"),
        fallback: extract_summary(&rows, "fallback"),
    };

    Ok(summary)
}
