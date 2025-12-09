use sqlx::PgPool;

pub mod models;
pub use models::*;

pub mod authors;
pub mod citations;
pub mod publication_authors;
pub mod publications;
pub mod users;

pub use authors::AuthorOperations;
pub use citations::CitationOperations;
pub use publication_authors::PublicationAuthorOperations;
pub use publications::PublicationOperations;
pub use users::UserOperations;

pub struct SqlClient {
    pub db: sqlx::PgPool,
}

impl SqlClient {
    pub async fn new(pool: PgPool) -> Self {
        Self { db: pool }
    }
}

pub type PrivyId = String;

#[cfg(test)]
pub mod tests;
