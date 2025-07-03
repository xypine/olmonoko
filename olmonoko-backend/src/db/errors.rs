#[derive(Debug, thiserror::Error)]
pub enum TemplateOrDatabaseError {
    #[error("Database error: {0}")]
    DbErr(#[from] sqlx::Error),
    #[error("Template rendering error: {0}")]
    TemplateErr(#[from] tera::Error),
}
