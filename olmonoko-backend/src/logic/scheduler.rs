use super::source_processing;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

async fn job_sync_all_sources(job_uuid: String) {
    tracing::info!(job_uuid, "Syncing all sources!");
    source_processing::sync_all()
        .await
        .expect("Failed to sync sources");
    tracing::info!("Sync complete!");
}

pub async fn init() -> Result<(), JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;
    // refetch calendar sources every 5 minutes
    scheduler
        .add(Job::new_async("0 */5 * * * *", |job_uuid, _| {
            Box::pin(async move {
                job_sync_all_sources(job_uuid.to_string()).await;
            })
        })?)
        .await?;

    scheduler.shutdown_on_ctrl_c();
    scheduler.start().await.unwrap();

    Ok(())
}
