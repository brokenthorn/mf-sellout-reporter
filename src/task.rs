//! # Job

use std::{future::Future, pin::Pin};

use chrono::{DateTime, Utc};
use cron::Schedule;
use futures::future::join_all;
use tracing::log::info;
use uuid::Uuid;

pub type TaskFuture<T> = Pin<Box<dyn Future<Output = T>>>;
pub type TaskGenerator<T> = Box<dyn Fn() -> TaskFuture<T>>;

pub struct Task<'a, T> {
    id: Uuid,
    name: &'a str,
    schedule: Schedule,
    pub last: Option<DateTime<Utc>>,
    // boxed because of the unsized nature of the dyn Fn() trait,
    // which means it cannot be a local variable in current Rust:
    pub task_generator: TaskGenerator<T>,
}

impl<'a, T> std::fmt::Debug for Task<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

impl<'a, T> Task<'a, T> {
    pub fn new(name: &'a str, schedule: Schedule, task_generator: TaskGenerator<T>) -> Self {
        Task {
            id: Uuid::new_v4(),
            name,
            schedule,
            last: None,
            task_generator,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name
    }
}

#[derive(Debug)]
pub struct TaskScheduler<'a, T>
where
    T: std::fmt::Debug,
{
    id: Uuid,
    name: &'a str,
    tasks: Vec<Task<'a, T>>,
}

impl<'a, T> TaskScheduler<'a, T>
where
    T: std::fmt::Debug,
{
    pub fn new(name: &'a str) -> Self {
        TaskScheduler {
            id: Uuid::new_v4(),
            name,
            tasks: Vec::new(),
        }
    }

    pub fn with_capacity(name: &'a str, capacity: usize) -> Self {
        TaskScheduler {
            id: Uuid::new_v4(),
            name,
            tasks: Vec::with_capacity(capacity),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn add(&mut self, task: Task<'a, T>) {
        self.tasks.push(task);
    }

    pub async fn start(&self) {
        info!("Starting {:?}", self);
        println!("Starting {:?}", self);

        loop {
            let now = Utc::now();
            let max_deviation_seconds = 10;

            let upcoming: Vec<(Uuid, DateTime<Utc>)> = self
                .tasks
                .iter()
                // 1. Get the next run times of the all the tasks we manage:
                .map(|task| (task.id(), task.schedule.after(&now).next()))
                .filter(|tpl| tpl.1.is_some())
                .map(|tpl| {
                    (
                        tpl.0,
                        tpl.1
                            .expect("The upcoming `Option<DateTime>` cannot be `None`!"),
                    )
                })
                // 2. Filter by the tasks that have start times within +/- `max_deviation_seconds` from now:
                .filter(|tpl: &(Uuid, DateTime<Utc>)| {
                    (tpl.1 - now).num_seconds().abs() < max_deviation_seconds
                })
                .collect();

            let workers: Vec<TaskFuture<T>> = self
                .tasks
                .iter()
                .filter(|task| upcoming.iter().any(|u| u.0 == task.id))
                .map(|task| (task.task_generator)())
                .collect();

            // 3. Start these tasks concurrently.
            //    `futures::future::join_all` can be used with anything that implements IntoIterator.

            info!("Starting tasks: {:?}", upcoming);
            println!("Starting tasks: {:?}", upcoming);

            let results = join_all(workers).await;

            info!("Results: {:?}", results);
            println!("Results: {:?}", results);

            // let job_futures: Vec<Pin<Box<dyn Future<Output = Result<T>>>>> =
            //     jobs.iter().map(|j| (j.task_generator)()).collect();

            // let results = futures::future::join_all(job_futures).await;

            // println!("Joined all futures for a result of: {:?}", results);

            // sleep(std::time::Duration::from_millis(
            //     self.tick_interval_ms.try_into().unwrap(),
            // ))
            // .await;

            // println!("Tick\n");

            info!("Finished running tasks: {:?}", upcoming);
            println!("Finished running tasks: {:?}", upcoming);
            async_std::task::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use async_std::task::sleep;
//     use std::time::Duration;

//     /// A future generator function that creates a future that takes 1 second to complete.
//     fn one_sec_future_generator() -> Pin<Box<dyn Future<Output = Result<&'static str>>>> {
//         Box::pin(async {
//             sleep(Duration::from_secs(1)).await;
//             println!("Worker that takes 1sec, finished.");
//             Ok("Worker that takes 1sec, finished.")
//         })
//     }

//     /// A future generator function that creates a future that takes 5 seconds to complete.
//     fn five_sec_future_generator() -> Pin<Box<dyn Future<Output = Result<&'static str>>>> {
//         Box::pin(async {
//             sleep(Duration::from_secs(5)).await;
//             println!("Worker that takes 5sec, finished.");
//             Ok("Worker that takes 5sec, finished.")
//         })
//     }

//     /// Tries to schedule a job that takes 1 second to complete, to run every 2 seconds.
//     /// This uses a job scheduler tick interval of 1 second.
//     #[test]
//     fn can_schedule_one_job() {
//         println!("Trying to schedule just 1 job...");

//         let job: Task<&str> = Task::new(
//             0,
//             "0/1 * * * * *".parse().unwrap(),
//             Box::new(one_sec_future_generator),
//         );
//         let mut scheduler = TaskScheduler::<&str>::new(500);
//         scheduler.add(job);
//         let _output = async_std::task::block_on(scheduler.start());
//     }

//     /// Tries to schedule 10 jobs that each take 5 seconds to complete, to run every 1 second.
//     /// This uses a job scheduler tick interval of 1 second.
//     #[test]
//     fn can_schedule_overlapping_jobs() {
//         println!("Trying to schedule overlapping jobs...");

//         let job1: Task<&str> = Task::new(
//             0,
//             "0/1 * * * * *".parse().unwrap(),
//             Box::new(five_sec_future_generator),
//         );
//         let job2: Task<&str> = Task::new(
//             0,
//             "0/1 * * * * *".parse().unwrap(),
//             Box::new(five_sec_future_generator),
//         );
//         let job3: Task<&str> = Task::new(
//             0,
//             "0/1 * * * * *".parse().unwrap(),
//             Box::new(five_sec_future_generator),
//         );
//         let mut scheduler = TaskScheduler::<&str>::new(500);
//         scheduler.add(job1);
//         scheduler.add(job2);
//         scheduler.add(job3);
//         let _output = async_std::task::block_on(scheduler.start());
//     }
// }
