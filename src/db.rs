use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

pub(crate) type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

pub(crate) type BB8PooledConnection<'a> =
    bb8::PooledConnection<'a, bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

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
