use crate::{payment_processors::structs::PaymentProcessorHealth, pubsub};
use redis::{Client, Commands, Connection, Msg, PubSub, RedisResult};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Subscriber {
    conn: Connection,
    channel_name: String,
}

#[derive(Clone)]
pub struct HealthCheckChannel {
    client: Client,
    channel_name: String,
    conn: Arc<Mutex<Connection>>,
}

impl Subscriber {
    pub fn new(conn: Connection, channel_name: &str) -> Self {
        Self {
            conn,
            channel_name: channel_name.to_string(),
        }
    }

    pub async fn next_message(&mut self) -> RedisResult<Msg> {
        let mut pubsub = self.conn.as_pubsub();
        pubsub.subscribe(&[self.channel_name.clone()])?;

        pubsub.get_message()
    }
}

impl HealthCheckChannel {
    pub fn new(socket: &str) -> RedisResult<Self> {
        let channel_name =
            std::env::var("HEALTH_CHECK_CHANNEL").unwrap_or_else(|_| "healthcheck".to_string());

        let client = Client::open(socket)?;
        let conn = client.get_connection()?;
        Ok(Self {
            client,
            channel_name,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn update(
        &self,
        msg: PaymentProcessorHealth,
    ) -> Result<(), bb8_redis::redis::RedisError> {
        let value = serde_json::to_string(&msg).map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::ParseError,
                "Serialization error",
                e.to_string(),
            ))
        })?;

        let _: Result<String, redis::RedisError> =
            self.conn.lock().await.publish(&self.channel_name, &value);

        Ok(())
    }

    pub fn subscribe(&self) -> RedisResult<Subscriber> {
        let conn = self.client.get_connection()?;

        let subs = Subscriber::new(conn, &self.channel_name);
        Ok(subs)
    }
}
