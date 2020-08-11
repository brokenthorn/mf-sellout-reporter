//! # IQVIA sellout reporter
//!
//! Provides a reporter for sellout data required by IQVIA from pharmacies in Romania.

use crate::job::{Job, CalendaristicJobScheduler};
use chrono::Local;
use tracing::info;

/// Error type for jobs inside the `iqvia` module.
pub type Error = String;

/// Job result type for jobs inside the `iqvia` module.
pub type JobResult = String;

/// Context object to be passed to each job when executing.
/// Use this to pass things such as database connection pools, or configuration values.
struct JobContext {
    /// Database connection pool.
    pub db_pool: String,
}

/// IQVIA Job scheduler that can schedule the creation of the reports requested by IQVIA.
///
/// It implements the [JobScheduler] trait.
pub struct IqviaJobScheduler {
    /// Report creation jobs vector.
    jobs: Vec<Box<dyn Job<JobResult, Error, JobContext, Local>>>,
}

impl CalendaristicJobScheduler for IqviaJobScheduler {
    fn start_job_scheduler(&self) -> anyhow::Result<()> {
        if self.jobs.is_empty() {
            bail!("Cannot start job scheduler with 0 jobs in queue.");
        }
        info!(
            "Starting job scheduler with {} jobs in queue.",
            self.jobs.len()
        );
        Ok(())
    }
}

impl IqviaJobScheduler {
    /// Creates a new job scheduler.
    pub fn new() -> Self {
        IqviaJobScheduler { jobs: vec![] }
    }
}
