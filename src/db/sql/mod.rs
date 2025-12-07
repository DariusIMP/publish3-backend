use sqlx::PgPool;

pub mod models;
pub use models::*;

pub mod users;
pub mod authors;
pub mod publications;
pub mod citations;
pub mod publication_authors;

pub use users::UserOperations;
pub use authors::AuthorOperations;
pub use publications::PublicationOperations;
pub use citations::CitationOperations;
pub use publication_authors::PublicationAuthorOperations;

pub struct SqlClient {
    pub db: sqlx::PgPool,
}

impl SqlClient {
    pub async fn new(pool: PgPool) -> Self {
        Self { db: pool }
    }
}
