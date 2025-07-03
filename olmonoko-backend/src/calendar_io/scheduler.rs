use super::source_processing;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

async fn job_sync_all_sources(job_uuid: String, oneoff: bool) {
    if oneoff {
        tracing::info!(job_uuid, "Syncing all sources! (oneoff)");
    } else {
        tracing::info!(job_uuid, "Syncing all sources!");
    }
    source_processing::sync_all()
        .await
        .expect("Failed to sync sources");
    tracing::info!(job_uuid, "Sync complete!");
}

pub async fn schedule_sync_oneoff(scheduler: &JobScheduler) -> Result<(), JobSchedulerError> {
    scheduler
        .add(Job::new_one_shot_async(
            std::time::Duration::from_secs(18),
            |job_uuid, _| {
                Box::pin(async move {
                    job_sync_all_sources(job_uuid.to_string(), true).await;
                })
            },
        )?)
        .await?;
    Ok(())
}

pub async fn init() -> Result<JobScheduler, JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;
    // refetch calendar sources every 5 minutes
    scheduler
        .add(Job::new_async("0 */5 * * * *", |job_uuid, _| {
            Box::pin(async move {
                job_sync_all_sources(job_uuid.to_string(), false).await;
            })
        })?)
        .await?;

    scheduler.shutdown_on_ctrl_c();
    scheduler.start().await.unwrap();

    Ok(scheduler)
}
