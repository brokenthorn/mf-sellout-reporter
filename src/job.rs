//! # Job

use std::future::Future;

use anyhow::Result;
use async_std::task::sleep;
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use tracing::log::info;
use uuid::Uuid;

/// A schedulable `Job`.
pub struct Job<T> {
    /// The job's unique identifier.
    id: Uuid,
    /// The job's priority when scheduled to run at the same time as other jobs on the same scheduler.
    /// Lower numbers mean higher priority.
    priority: usize,
    /// The job's schedule.
    schedule: Schedule,
    /// A function that returns a `Future` containing this job's task
    /// so that the `Future` can be constructed and called multiple times, depending on how
    /// this `Job` will be scheduled.
    run: Box<dyn Fn() -> Box<dyn Future<Output = Result<T>>>>,
}

impl<T> Job<T> {
    /// Creates a new `Job` instance.
    pub fn new(
        priority: usize,
        schedule: Schedule,
        run: Box<dyn Fn() -> Box<dyn Future<Output = Result<T>>>>,
    ) -> Self {
        Job {
            id: Uuid::new_v4(),
            schedule,
            priority,
            run,
        }
    }
}

impl<T> std::fmt::Debug for Job<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("run", &"Fn() -> Box<dyn Future>")
            .field("schedule", &"cron::Schedule")
            .finish()
    }
}

#[derive(Debug)]
struct JobStats {
    /// The job's unique identifier.
    id: Uuid,
    /// The number of times this job has failed to run on time.
    pub misses: usize,
    /// The number of times this job has ran successfully.
    pub runs: usize,
    /// The job's last successful run.
    pub last_run: Option<DateTime<Utc>>,
}

impl JobStats {
    /// Creates a new `JobStats` instance.
    pub fn new(id: Uuid) -> Self {
        JobStats {
            id,
            misses: 0,
            runs: 0,
            last_run: None,
        }
    }

    /// Returns the job's unique identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }
}

/// A job scheduler for running `Job`s that produce generic `T` values when finished.
#[derive(Default)]
pub struct JobScheduler<T> {
    /// The jobs that this `JobScheduler` schedules.
    jobs: Vec<Job<T>>,
    /// Statistics for the jobs that this `JobScheduler` schedules.
    job_stats: Vec<JobStats>,
    /// The interval of time, in seconds, that this scheduler checks for
    /// any jobs that it should run.
    ///
    /// The highest precision possible is 1 second but if you don't need that level of precission,
    /// setting this property to a higher value can lower CPU usage.
    tick_interval_sec: u8,
}

impl<T> JobScheduler<T> {
    /// Creates a new `JobScheduler` instance.
    pub fn new(tick_interval_sec: u8) -> Self {
        JobScheduler {
            jobs: Vec::new(),
            job_stats: Vec::new(),
            tick_interval_sec,
        }
    }

    /// Adds a new job to this scheduler.
    pub fn add(&mut self, job: Job<T>) {
        self.jobs.push(job);
    }

    /// Starts the job scheduler.
    ///
    /// This function is asynchronous so it returns a `Future`
    /// which should be pooled (by an executor) or awaited, otherwise no work will be done.
    pub async fn start(&self) {
        info!("Scheduler starting");

        loop {
            println!("Loop!");

            // allow a max deviation equal to scheduler's tick interval + 1 seconds:
            let allowed_deviation_duration = Duration::seconds((self.tick_interval_sec + 1).into());
            let allowed_deviation_seconds = allowed_deviation_duration.num_seconds();
            let now = Utc::now();

            let jobs: Vec<&Job<T>> = self
                .jobs
                .iter()
                .filter(|j| {
                    // if the upcoming job's execution time is within the allowed deviation,
                    // we should run it.

                    if let Some(upcoming) =
                        j.schedule.after(&(now - allowed_deviation_duration)).next()
                    {
                        let abs_deviation_seconds = (now - upcoming).num_seconds().abs();
                        if abs_deviation_seconds <= allowed_deviation_seconds {
                            return true;
                        }
                    }
                    false
                })
                .collect();

            info!("Found some jobs that should run now: {:?}", jobs);
            println!("Found some jobs that should run now: {:?}", jobs);

            // TODO: 1. Check if any jobs have more than limit_missed_runs missed runs
            //   and run those async tasks now with a helper fr

            sleep(std::time::Duration::from_secs(
                self.tick_interval_sec.into(),
            ))
            .await;
            
            info!("Tick");
            println!("Tick");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_std::task::sleep;
    use std::time::Duration;

    /// Creates a simple worker future that waits for 1 second before finishing.
    fn create_future_that_takes_1sec() -> Box<dyn Future<Output = Result<&'static str>>> {
        Box::new(async {
            sleep(Duration::from_secs(1)).await;
            println!("Worker that takes 1sec, finished.");
            Ok("Worker that takes 1sec, finished.")
        })
    }

    /// Creates a simple worker future that waits for 5 seconds before finishing.
    fn create_future_that_takes_5sec() -> Box<dyn Future<Output = Result<&'static str>>> {
        Box::new(async {
            sleep(Duration::from_secs(5)).await;
            println!("Worker that takes 5sec, finished.");
            Ok("Worker that takes 5sec, finished.")
        })
    }

    /// Tries to schedule a job that takes 1 second to complete, to run every 2 seconds.
    /// This uses a job scheduler tick interval of 1 second.
    #[test]
    fn can_schedule_one_job() {
        println!("Trying to schedule just 1 job...");

        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "trace");
        }

        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
            // will be written to stdout.
            .with_max_level(tracing::Level::TRACE)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default log subscriber failed");

        let job: Job<&str> = Job::new(
            0,
            "0/1 * * * * *".parse().unwrap(),
            Box::new(create_future_that_takes_1sec),
        );
        let mut scheduler = JobScheduler::<&str>::new(1);
        scheduler.add(job);
        let _output = async_std::task::block_on(scheduler.start());
    }

    /// Tries to schedule 10 jobs that each take 5 seconds to complete, to run every 1 second.
    /// This uses a job scheduler tick interval of 1 second.
    #[test]
    fn can_schedule_overlapping_jobs() {
        let job: Job<&str> = Job::new(
            0,
            "0/1 * * * * *".parse().unwrap(),
            Box::new(create_future_that_takes_5sec),
        );
    }
}
