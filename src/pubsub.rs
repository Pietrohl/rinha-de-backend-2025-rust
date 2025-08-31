use crate::payment_processors::structs::PaymentProcessorHealth;
use redis::{Client, Commands, Connection, Msg, PubSub, RedisResult};
use std::sync::Arc;
use tokio::sync::Mutex;



pub struct Subscriber {
    client: Client,
    conn: Arc<Mutex<Connection>>,
    channel_name: String,
}

#[derive(Clone)]
pub struct HealthCheckChannel {
    client: Client,
    channel_name: String,
    conn: Arc<Mutex<Connection>>,
}


impl Subscriber {
    async fn get_pubsub(&self) -> RedisResult<PubSub<'static>> {
        
        let mut conn = self.conn.lock().await;
        
        let boxed_conn: Box<Connection> = Box::new(std::mem::replace(
            &mut *conn,
            self.client.get_connection()?,
        ));
        let conn_ref: &'static mut Connection = Box::leak(boxed_conn);
        let mut pubsub = conn_ref.as_pubsub();
        pubsub.subscribe(&[&self.channel_name])?;
        Ok(pubsub)
    }

    pub async fn next_message(&self) -> RedisResult<Msg> {
        let mut pubsub = self.get_pubsub().await?;
        println!("Pubsub created");
        let msg = pubsub.get_message()?;
        println!("Message received");
        Ok(msg)
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

    pub fn subscribe(&self) -> Subscriber {
        Subscriber {
            client: self.client.clone(),
            conn: Arc::clone(&self.conn),
            channel_name: self.channel_name.clone(),
        }
    }
}
