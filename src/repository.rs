use std::error::Error;

use crate::{
    db::{MemoryDatabase, PostgresDatabase},
    structs::{PaymentDatabaseEntry, PaymentsServiceSummary},
};
use redis::RedisError;
use tokio_postgres::Row;

use chrono::{DateTime, Utc};

use crate::{
    db::PostgresPooledConnection, payment_processors, structs::PaymentsSummaryResponseDTO,
};

// const INSERT_QUERY: &str = "INSERT INTO transactions (correlation_id, processed_at, amount, service) VALUES ($1, $2, $3, $4)";

const SUMMARY_QUERY: &str = "SELECT service, COUNT(*) as total_requests, CAST(COALESCE(SUM(amount), 0) as BIGINT) as total_amount FROM transactions WHERE ($1::timestamptz IS NULL OR processed_at >= $1) AND ($2::timestamptz IS NULL OR processed_at <= $2) GROUP BY service";

fn extract_summary(rows: &[Row], service: &str) -> PaymentsServiceSummary {
    for row in rows {
        let row_service: String = row.get("service");
        if row_service == service {
            let total_requests: i64 = row.get("total_requests");
            let total_amount_dec: i64 = row.get("total_amount");
            let mut total_amount: f64 = total_amount_dec as f64; // Convert cents to dollars
            total_amount /= 100.00; // Convert cents to dollars
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

// fn extract_memory_summary(memory_payments: &[PaymentDatabaseEntry]) -> PaymentsSummaryResponseDTO {
//     let mut default_total_requests = 0u32;
//     let mut default_total_amount = 0f64;
//     let mut fallback_total_requests = 0u32;
//     let mut fallback_total_amount = 0f64;

//     for entry in memory_payments {
//         match entry.service {
//             PaymentProcessorServices::Default => {
//                 default_total_requests += 1;
//                 default_total_amount += entry.amount;
//             }
//             PaymentProcessorServices::Fallback => {
//                 fallback_total_requests += 1;
//                 fallback_total_amount += entry.amount;
//             }
//             _ => {}
//         }
//     }

//     PaymentsSummaryResponseDTO {
//         default: PaymentsServiceSummary {
//             total_requests: default_total_requests,
//             total_amount: default_total_amount,
//         },
//         fallback: PaymentsServiceSummary {
//             total_requests: fallback_total_requests,
//             total_amount: fallback_total_amount,
//         },
//     }
// }

pub async fn save_processed_payment(
    mem_db: &MemoryDatabase,
    correlation_id: uuid::Uuid,
    date: DateTime<Utc>,
    amount: f64,
    service: &payment_processors::service::PaymentProcessorServices,
) -> Result<(), RedisError> {
    mem_db
        .insert(&format!(
            "{}|{}|{}|{}",
            correlation_id,
            date.to_rfc3339(),
            (amount * 100.0).round() as i64,
            service.to_string()
        ))
        .await?;
    Ok(())
}

pub async fn get_payments_summary<'a>(
    memory_database: &MemoryDatabase,
    db: &PostgresDatabase,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<PaymentsSummaryResponseDTO, Box<dyn Error>> {
    let result = memory_database
        .pop_all()
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;

    let memory_payments: Vec<PaymentDatabaseEntry> = result
        .into_iter()
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split('|').collect();
            if parts.len() != 4 {
                return None;
            }
            let correlation_id = uuid::Uuid::parse_str(parts[0]).ok()?;
            let requested_at = DateTime::parse_from_rfc3339(parts[1])
                .ok()?
                .with_timezone(&Utc);
            let amount_cents = parts[2].parse::<i64>().ok()?;
            let amount = amount_cents as f64 / 100.0; // Convert cents to dollars
            let service = payment_processors::service::PaymentProcessorServices::from(parts[3]);
            Some(PaymentDatabaseEntry {
                correlation_id,
                requested_at,
                amount,
                service,
            })
        })
        .collect();



    let conn = db.pool.get().await.map_err(|e| {
        Box::new(e) as Box<dyn Error>
    })?;

    if !memory_payments.is_empty() {
        
        let mut query = String::from("INSERT INTO transactions (correlation_id, processed_at, amount, service) VALUES ");
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut placeholders = Vec::new();


        let amount_cents_vec: Vec<i64> = memory_payments.iter().map(|entry| (entry.amount * 100.0).round() as i64).collect();
        let service_str_vec: Vec<String> = memory_payments.iter().map(|entry| entry.service.to_string()).collect();
        for (i, entry) in memory_payments.iter().enumerate() {
            let base = i * 4;
            placeholders.push(format!("(${}, ${}, ${}, ${})", base + 1, base + 2, base + 3, base + 4));
            params.push(&entry.correlation_id);
            params.push(&entry.requested_at);
            params.push(&amount_cents_vec[i]);
            params.push(&service_str_vec[i]);
        }
        query.push_str(&placeholders.join(", "));
        let _ = conn.execute(query.as_str(), &params).await?;
    }

    let rows = conn.query(SUMMARY_QUERY, &[&from, &to]).await?;

    let summary = PaymentsSummaryResponseDTO {
        default: extract_summary(&rows, "default"),
        fallback: extract_summary(&rows, "fallback"),
    };

    Ok(summary)
}

pub async fn purge_payments<'a>(conn: PostgresPooledConnection<'a>) -> Result<u64, Box<dyn Error>> {
    let query = "DELETE FROM transactions";
    let rows_affected = conn.execute(query, &[]).await?;
    Ok(rows_affected)
}
