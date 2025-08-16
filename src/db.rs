
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::{NoTls, Row};

pub(crate) type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

pub(crate) type BB8PooledConnection<'a> =
    bb8::PooledConnection<'a, bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

pub trait DBConnection<'a> {
    async fn query_extrato(&self, id: i32) -> Result<Vec<Row>, tokio_postgres::Error>;
    async fn insert_transaction(
        &self,
        id: i32,
        value: i32,
        transaction_type: &str,
        description: &str,
    ) -> Result<Row, tokio_postgres::Error>;
}

pub(crate) struct PostgresConnection<'a> {
    conn: BB8PooledConnection<'a>,
}

impl<'a> PostgresConnection<'a> {
    pub fn new(conn: BB8PooledConnection<'a>) -> Self {
        Self { conn }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresDatabase {
    pub pool: ConnectionPool,
}

impl PostgresDatabase {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

pub(crate) const DATABASE_URL: &str =
    "host=db port=5432 password=postgres user=postgres dbname=rinha_2025_db";
