#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("cannot open db: {0}")]
    CannotOpenDb(String),

    #[error("sql error: {0}")]
    Sql(#[from] rusqlite::Error),
}
