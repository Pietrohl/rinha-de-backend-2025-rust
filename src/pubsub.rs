use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;


struct HealthCheckChannel {
     pool: RedisQueueConnection,
    channel_name: String,
    publisher: 
}
pub(crate) type RedisChannelConnection = Pool<RedisConnectionManager>;

impl HealthCheckChannel {
     pub fn new(pool: RedisChannelConnection) -> Self {
        let channel_name =
            std::env::var("HEALTH_CHECK_CHANNEL").unwrap_or_else(|_| "healthcheck".to_string());

        Self {
            pool,
            channel_name,
            publisher
        }
    }



    async pub update(self, msg: PaymentProcessorHealth) -> Result<(), bb8_redis::redis::RedisError> {

        let mut conn = self.pool.get().await.map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::IoError,
                "bb8 pool error",
                e.to_string(),
            ))
        })?;


        let value = serde_json::to_string(&msg).map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::ParseError,
                "Serialization error",
                e.to_string(),
            ))
        })?;


        conn.publish(self.channel_name, value).await?;
        OK(())
    }


    async pub subscribe(self) -> Result<redis::PubSub, bb8_redis::redis::RedisError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::IoError,
                "bb8 pool error",
                e.to_string(),
            ))
        })?;

        let mut pubsub = con.as_pubsub();
        pubsub.subscribe(&[self.channel_name])?;

        pubsub
    }

}