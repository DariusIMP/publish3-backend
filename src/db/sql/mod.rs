use sqlx::PgPool;

pub struct SqlClient {
    pub db: sqlx::PgPool,
}

impl SqlClient {
    pub async fn new(pool: PgPool) -> Self {
        Self { db: pool }
    }
}
