//! # Job

use std::{convert::TryInto, future::Future, pin::Pin};

use anyhow::Result;
use async_std::task::sleep;
use chrono::{DateTime, Duration, SubsecRound, Utc};
use cron::Schedule;
use tracing::log::{info, warn};
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
    run: Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<T>>>>>,
}

impl<T> Job<T> {
    /// Creates a new `Job` instance.
    pub fn new(
        priority: usize,
        schedule: Schedule,
        run: Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<T>>>>>,
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
        f.debug_struct("Job").field("id", &self.id).finish()
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
pub struct JobScheduler<T>
where
    T: std::fmt::Debug,
{
    /// The jobs that this `JobScheduler` schedules.
    jobs: Vec<Job<T>>,
    /// Statistics for the jobs that this `JobScheduler` schedules.
    job_stats: Vec<JobStats>,
    /// The interval of time, in milliseconds, that this scheduler checks for
    /// any jobs that it should run.
    ///
    /// Setting this property to a higher value can lower CPU usage!
    tick_interval_ms: i64,
}

impl<T> JobScheduler<T>
where
    T: std::fmt::Debug,
{
    /// Creates a new `JobScheduler` instance.
    pub fn new(tick_interval_ms: i64) -> Self {
        JobScheduler {
            jobs: Vec::new(),
            job_stats: Vec::new(),
            tick_interval_ms,
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

        let max_miss_by_duration = Duration::milliseconds(self.tick_interval_ms);
        let max_miss_by_seconds = max_miss_by_duration.num_seconds();

        loop {
            let now = Utc::now().round_subsecs(0);

            let jobs: Vec<&Job<T>> = self
                .jobs
                .iter()
                .filter(|j| {
                    // if the upcoming execution time is ± allowed_deviation, we should run it:
                    if let Some(upcoming) =
                        j.schedule.after(&(now - max_miss_by_duration)).next()
                    {
                        println!("Found job that should run: {:?}", j);

                        let deviation_seconds = (now - upcoming).num_seconds();
                        let abs_deviation_seconds = deviation_seconds.abs();

                        if abs_deviation_seconds > 0 {
                            println!(
                                "Deviated from {} job execution time by {} seconds from allowed {}! Now={}, Scheduled={}.",
                                j.id,
                                deviation_seconds,
                                max_miss_by_seconds,
                                now,
                                upcoming
                            );
                            warn!(
                                "Deviated from {} job execution time by {} seconds from allowed {}! Now={}, Scheduled={}.",
                                j.id,
                                deviation_seconds,
                                max_miss_by_seconds,
                                now,
                                upcoming
                            );
                        }

                        if abs_deviation_seconds <= max_miss_by_seconds {
                            return true;
                        }
                    }
                    false
                })
                .collect();

            let job_futures: Vec<Pin<Box<dyn Future<Output = Result<T>>>>> =
                jobs.iter().map(|j| (j.run)()).collect();

            let results = futures::future::join_all(job_futures).await;

            println!("Joined all futures for a result of: {:?}", results);

            sleep(std::time::Duration::from_millis(
                self.tick_interval_ms.try_into().unwrap(),
            ))
            .await;

            println!("Tick\n");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_std::task::sleep;
    use std::time::Duration;

    /// A future generator function that creates a future that takes 1 second to complete.
    fn one_sec_future_generator() -> Pin<Box<dyn Future<Output = Result<&'static str>>>> {
        Box::pin(async {
            sleep(Duration::from_secs(1)).await;
            println!("Worker that takes 1sec, finished.");
            Ok("Worker that takes 1sec, finished.")
        })
    }

    /// A future generator function that creates a future that takes 5 seconds to complete.
    fn five_sec_future_generator() -> Pin<Box<dyn Future<Output = Result<&'static str>>>> {
        Box::pin(async {
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

        let job: Job<&str> = Job::new(
            0,
            "0/1 * * * * *".parse().unwrap(),
            Box::new(one_sec_future_generator),
        );
        let mut scheduler = JobScheduler::<&str>::new(500);
        scheduler.add(job);
        let _output = async_std::task::block_on(scheduler.start());
    }

    /// Tries to schedule 10 jobs that each take 5 seconds to complete, to run every 1 second.
    /// This uses a job scheduler tick interval of 1 second.
    #[test]
    fn can_schedule_overlapping_jobs() {
        println!("Trying to schedule overlapping jobs...");

        let job1: Job<&str> = Job::new(
            0,
            "0/1 * * * * *".parse().unwrap(),
            Box::new(five_sec_future_generator),
        );
        let job2: Job<&str> = Job::new(
            0,
            "0/1 * * * * *".parse().unwrap(),
            Box::new(five_sec_future_generator),
        );
        let job3: Job<&str> = Job::new(
            0,
            "0/1 * * * * *".parse().unwrap(),
            Box::new(five_sec_future_generator),
        );
        let mut scheduler = JobScheduler::<&str>::new(500);
        scheduler.add(job1);
        scheduler.add(job2);
        scheduler.add(job3);
        let _output = async_std::task::block_on(scheduler.start());
    }
}
