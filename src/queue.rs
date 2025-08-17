use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;

use crate::{payment_processors::structs::PaymentProcessorDTO};

pub(crate) type RedisQueueConnection = Pool<RedisConnectionManager>;

#[derive(Debug, Clone)]
pub struct RedisQueue {
    pool: RedisQueueConnection,
    collection_name: String,
}

impl RedisQueue {
    pub fn new(pool: RedisQueueConnection) -> Self {
        let collection_name =
            std::env::var("REDIS_QUEUE_NAME").unwrap_or_else(|_| "payments_queue".to_string());

        Self {
            pool,
            collection_name,
        }
    }

    pub async fn push(
        &self,
        payment: PaymentProcessorDTO,
    ) -> Result<(), bb8_redis::redis::RedisError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::IoError,
                "bb8 pool error",
                e.to_string(),
            ))
        })?;

        let value = serde_json::to_string(&payment).map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::ParseError,
                "Serialization error",
                e.to_string(),
            ))
        })?;

        let _: () = AsyncCommands::lpush(&mut *conn, &self.collection_name, value).await?;
        Ok(())
    }

    pub async fn pop(&self) -> Result<Option<PaymentProcessorDTO>, bb8_redis::redis::RedisError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::IoError,
                "bb8 pool error",
                e.to_string(),
            ))
        })?;

        let value: Option<String> = AsyncCommands::rpop(&mut *conn, &self.collection_name, None).await?;
        
        if let Some(value) = value {
            let payment: PaymentProcessorDTO = serde_json::from_str(&value).map_err(|e| {
                bb8_redis::redis::RedisError::from((
                    bb8_redis::redis::ErrorKind::ParseError,
                    "Deserialization error",
                    e.to_string(),
                ))
            })?;
            Ok(Some(payment))
        } else {
            Ok(None)
        }
    }


}
