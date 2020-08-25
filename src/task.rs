//! # Job

use std::{future::Future, pin::Pin};

use chrono::{DateTime, Utc};
use cron::Schedule;
use futures::future::join_all;
use tracing::log::{error, info};
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

    pub async fn start(&self, max_deviation_seconds: i64) {
        info!("Starting {:?}", self);
        println!("Starting {:?}", self);

        if max_deviation_seconds < 1 {
            error!("max_deviation_seconds is not allowed smaller than 1 seconds!");
            println!("max_deviation_seconds is not allowed smaller than 1 seconds!");
            return;
        }

        loop {
            let now = Utc::now();

            // 1. Get the next schedules as an iterator:

            let schedules_iter = self
                .tasks
                .iter()
                // Get the closest next run time of the all the tasks we manage:
                .map(|task| (task.id(), task.schedule.after(&now).next()))
                // Filter out those tasks that have no next run time (one-shot tasks that have already ran):
                // TODO: Write a cleanup function that removes these tasks that are one-shot and have no future run time,
                //       and call it at the start of the loop, then remove this filter step.
                .filter(|tpl| tpl.1.is_some())
                // Unwrap the Option<DateTime> for the upcoming run time:
                .map(|tpl| {
                    (
                        tpl.0,
                        tpl.1
                            .expect("The upcoming `Option<DateTime>` cannot be `None`!"),
                    )
                });

            // 2. Pick out from the next schedules iterator, the tasks that need to be run right now:

            let schedules_to_run_now: Vec<(Uuid, DateTime<Utc>)> = schedules_iter
                .clone()
                // Filter by the tasks that have start times within +/- `max_deviation_seconds` from now:
                .filter(|tpl: &(Uuid, DateTime<Utc>)| {
                    (tpl.1 - now).num_seconds().abs() < max_deviation_seconds
                })
                .collect();

            // 3. If no schedules were found that need to be run right now,
            //    then sleep until the next schedule that needs to run.
            //
            //    If any schedules that need to run right now were found, then run them.

            if schedules_to_run_now.is_empty() {
                let next_schedule_option: Option<DateTime<Utc>> = schedules_iter
                    // then find the earliest schedule:
                    .fold(None, |acc, t| {
                        if let Some(d) = acc {
                            if d < t.1 {
                                Some(d)
                            } else {
                                Some(t.1)
                            }
                        } else {
                            None
                        }
                    });

                if let Some(next_schedule) = next_schedule_option {
                    let dur = next_schedule - now;

                    info!(
                        "Sleeping for {:?} until the next schedule needs to run, which is {}",
                        dur, next_schedule
                    );
                    println!(
                        "Sleeping for {:?} until the next schedule needs to run, which is {}",
                        dur, next_schedule
                    );

                    async_std::task::sleep(
                        dur.to_std()
                            .expect("next_schedule shouldn't have been in the past!"),
                    )
                    .await;
                } else {
                    info!("There were no more tasks scheduled to run. Stoping scheduler.");
                    println!("There were no more tasks scheduled to run. Stoping scheduler.");
                    break;
                }
            } else {
                info!("Starting tasks: {:?}", schedules_to_run_now);
                println!("Starting tasks: {:?}", schedules_to_run_now);

                // generate task futures:
                let workers: Vec<TaskFuture<T>> = self
                    .tasks
                    .iter()
                    .filter(|task| {
                        schedules_to_run_now
                            .iter()
                            .any(|schedule| schedule.0 == task.id)
                    })
                    .map(|task| (task.task_generator)())
                    .collect();
                // run task futures to completion in parallel:
                let results = join_all(workers).await;

                info!("Finished running tasks: {:?}", schedules_to_run_now);
                println!("Finished running tasks: {:?}", schedules_to_run_now);

                info!("Results: {:?}", results);
                println!("Results: {:?}", results);
            }
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
