use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use bb8_redis::RedisConnectionManager;
use bb8_redis::redis::AsyncCommands;
use tokio_postgres::NoTls;

pub(crate) type PostgresConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

pub(crate) type PostgresPooledConnection<'a> =
    bb8::PooledConnection<'a, bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

#[derive(Debug, Clone)]
pub(crate) struct PostgresDatabase {
    pub pool: PostgresConnectionPool,
}

impl PostgresDatabase {
    pub(crate) fn new(pool: PostgresConnectionPool) -> Self {
        Self { pool }
    }
}

pub(crate) type MemoryDatabaseConnection = Pool<RedisConnectionManager>;

#[derive(Debug, Clone)]
pub struct MemoryDatabase {
    pub pool: MemoryDatabaseConnection,
    collection_name: String,
}

impl MemoryDatabase {
    pub fn new(pool: MemoryDatabaseConnection) -> Self {
        let collection_name = std::env::var("MEMORY_DATABASE_COLLECTION_NAME")
            .unwrap_or_else(|_| "payments".to_string());

        Self {
            pool,
            collection_name,
        }
    }

    pub async fn insert(&self, value: &str) -> Result<(), bb8_redis::redis::RedisError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::IoError,
                "bb8 pool error",
                e.to_string(),
            ))
        })?;
        let _: () = AsyncCommands::lpush(&mut *conn, &self.collection_name, value).await?;
        Ok(())
    }

    pub async fn pop_all(&self) -> Result<Vec<String>, bb8_redis::redis::RedisError> {
        use bb8_redis::redis::pipe;
        let mut conn = self.pool.get().await.map_err(|e| {
            bb8_redis::redis::RedisError::from((
                bb8_redis::redis::ErrorKind::IoError,
                "bb8 pool error",
                e.to_string(),
            ))
        })?;

        let mut pipeline = pipe();
        pipeline
            .lrange(&self.collection_name, 0, -1)
            .del(&self.collection_name);
        let result: (Vec<String>, ()) = pipeline.query_async(&mut *conn).await?;
        Ok(result.0)
    }

    // pub async fn get_all(&self) -> Result<Vec<String>, bb8_redis::redis::RedisError> {
    //     let mut conn = self.pool.get().await.map_err(|e| {
    //         bb8_redis::redis::RedisError::from((
    //             bb8_redis::redis::ErrorKind::IoError,
    //             "bb8 pool error",
    //             e.to_string(),
    //         ))
    //     })?;
    //     let result: Vec<String> = AsyncCommands::lrange(&mut *conn, &self.collection_name, 0, -1).await?;
    //     Ok(result)
    // }

}

pub(crate) const DEFAULT_DATABASE_URL: &str =
    "host=db port=5432 password=postgres user=postgres dbname=rinha_2025_db";

pub(crate) const DEFAULT_MEMORY_DATABASE_URL: &str = "redis://localhost:6379";
