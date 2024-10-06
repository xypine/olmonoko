use olmonoko_common::models::{
    ics_source::{IcsSource, RawIcsSource},
    subscription::Subscription,
};
use sqlx::{Executor, Postgres};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Source not found")]
    SourceNotFound,
    #[error("Failed to insert events: {0}")]
    InsertEventsError(#[from] sqlx::Error),
}
pub(crate) async fn sync_subscription<C>(
    conn: &mut C,
    subscription: Subscription,
) -> Result<bool, SyncError>
where
    for<'e> &'e mut C: Executor<'e, Database = Postgres>,
{
    let source = sqlx::query_as!(
        RawIcsSource,
        "SELECT * FROM ics_sources WHERE id = $1",
        subscription.ics_source_id
    )
    .fetch_one(&mut *conn)
    .await
    .map(|raw| (raw, None))
    .map(IcsSource::from)
    .map_err(|_| SyncError::SourceNotFound)?;

    todo!()
}
