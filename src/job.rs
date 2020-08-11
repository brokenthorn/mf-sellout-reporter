//! # Job

use std::future::Future;

use anyhow::Result;
use chrono::{DateTime, Utc};
use cron::Schedule;
use uuid::Uuid;

/// A schedulable `Job`.
pub struct Job<T> {
    /// The job's unique identifier.
    id: Uuid,
    /// The job's schedule.
    schedule: Schedule,
    /// The number of acceptable missed runs. 0 means unlimited.
    limit_missed_runs: usize,
    /// The job's last successful run.
    last_run: Option<DateTime<Utc>>,
    /// The number of times this job has ran successfully.
    runs: usize,
    /// The number of times this job has failed to run on time.
    misses: usize,
    /// A future for the completion of this job's task.
    run: Box<dyn FnMut() -> Box<dyn Future<Output = Result<T>>>>,
}

impl<T> Job<T> {
    /// Creates a new `Job`.
    pub fn new(
        run: Box<dyn FnMut() -> Box<dyn Future<Output = Result<T>>>>,
        schedule: Schedule,
        limit_missed_runs: usize,
    ) -> Self {
        Job {
            id: Uuid::new_v4(),
            schedule,
            limit_missed_runs,
            last_run: None,
            runs: 0,
            misses: 0,
            run,
        }
    }

    /// Increases the number of missed runs by 1.
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// Returns the number of missed runs this job has had.
    pub fn misses(&self) -> usize {
        self.misses
    }

    /// Returns the number of succesful runs this job has had.
    pub fn runs(&self) -> usize {
        self.runs
    }

    /// Returns this job's last successful run date and time.
    pub fn last_run(&self) -> Option<DateTime<Utc>> {
        self.last_run
    }
}

/// A job scheduler for running `Job`s.
#[derive(Default)]
pub struct JobScheduler<T> {
    jobs: Vec<Job<T>>,
}

impl<T> JobScheduler<T> {
    /// Starts the job scheduler.
    ///
    /// This function is asynchronous so it returns a `Future`
    /// which should be pooled (by an executor) or awaited, otherwise no work will be done.
    pub async fn start(&self) {}
}

#[cfg(test)]
mod test {
    use super::*;
    use async_std::task::sleep;
    use std::time::Duration;

    /// Returns a pointer to a simple worker future that is created on the heap when called.
    ///
    /// The future simply waits for 1 second before finishing by returning a `&str`.
    fn worker() -> Box<dyn Future<Output = Result<&'static str>>> {
        Box::new(async {
            sleep(Duration::from_secs(1)).await;
            Ok("Job finished running.")
        })
    }

    /// Tries to schedule a job to run every 2 second and stops after the 3rd run.
    #[test]
    fn can_schedule_one_job() {
        let job: Job<&str> = Job::new(Box::new(worker), "2".parse().unwrap(), 1);
    }
}
