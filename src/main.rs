#![forbid(unsafe_code)]
#![forbid(deprecated_in_future)]
#![warn(missing_docs)]

//! # MF Sellout Reporter
//!
//! A service purposefully built for the periodic reporting of sellout data to contract research
//! organizations or [CRO's](https://en.wikipedia.org/wiki/Contract_research_organization)
//! from the health information technology industry.
//!
//! ## How does it work?
//!
//! `mf-sellout-reporter` uses a task scheduler to run arbitrary jobs at periodic intervals.
//!
//! It supports `cron` syntax to define the task schedules, making it easy to express time
//! intervals, specific dates and/or periods.
//!
//! The under-the-hood task scheduler is actually a custom async task scheduler written initially
//! for `mf-sellout-reporter`, which aims to be as efficient as possible by sleeping in the interval
//! between schedules, effectively consuming no CPU resources in that time, but it can miss the
//! exact scheduled times by a few seconds, maybe more, especially if the system goes to sleep.
//!
//! ## How are tasks created?
//!
//! Tasks are hard-coded into the application binary. At the moment, there is no way of adding tasks
//! dynamically at runtime. If you need to run your own custom tasks, you will need to code them in
//! and recompile the application.

// pub mod iqvia;
pub mod task;

// #[macro_use]
extern crate anyhow;

use std::{future::Future, pin::Pin};

use anyhow::Result;
use task::{Task, TaskScheduler};
use tracing::{instrument, info, Level};
use tracing_subscriber::FmtSubscriber;

/// A future generator function that creates a future that takes 5 seconds to complete.
#[instrument]
fn five_sec_future_generator() -> Pin<Box<dyn Future<Output = Result<&'static str>>>> {
    Box::pin(async {
        async_std::task::sleep(std::time::Duration::from_secs(5)).await;
        Ok("Worker that takes 5sec, finished.")
    })
}

/// A future generator function that creates a future that takes 15 seconds to complete.
#[instrument]
fn fifteen_sec_future_generator() -> Pin<Box<dyn Future<Output = Result<&'static str>>>> {
    Box::pin(async {
        async_std::task::sleep(std::time::Duration::from_secs(15)).await;
        Ok("Worker that takes 15sec, finished.")
    })
}

#[instrument]
#[async_std::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "trace");
    }

    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default log subscriber failed");

    info!("Starting.");

    let job1: Task<Result<&str>> = Task::new(
        "Dummy 1",
        "0/7 * * * * *".parse().unwrap(),
        Box::new(five_sec_future_generator),
    );
    let job2: Task<Result<&str>> = Task::new(
        "Dummy 2",
        "0/3 * * * * *".parse().unwrap(),
        Box::new(five_sec_future_generator),
    );
    let job3: Task<Result<&str>> = Task::new(
        "Dummy 3",
        "0/7 * * * * *".parse().unwrap(),
        Box::new(fifteen_sec_future_generator),
    );
    let mut scheduler = TaskScheduler::<'_, Result<&str>>::with_capacity("Main Scheduler", 3);
    scheduler.add(job1);
    scheduler.add(job2);
    scheduler.add(job3);

    let _output = scheduler.start(1).await;

    // let iqvia_scheduler = iqvia::IqviaJobScheduler::new();
    // iqvia_scheduler.start()

    Ok(())
}

// use std::env;

// use async_std::net::TcpStream;
// use tiberius::{AuthMethod, Client, Config};

// // pub type DatabasePool = r2d2::Pool<MssqlConnectionManager<mssql::MssqlConnection>>;
// // pub type PooledConnection = r2d2::PooledConnection<CM<mss:MssqlConnection>>;
// pub struct Dummy;
// pub type DatabasePool = Dummy;

// pub struct JobContext {
//     db_pool: DatabasePool,
// }

// #[async_std::main]
// async fn main() -> anyhow::Result<()> {
//     let mut using_default_rust_log = false;
//     if env::var("RUST_LOG").is_err() {
//         using_default_rust_log = true;
//         env::set_var("RUST_LOG", "warn,statistics_reporter_svc=debug");
//     }
//     env_logger::init();
//     if using_default_rust_log {
//         info!("RUST_LOG env var was not set. Defaulting to `RUST_LOG=warn,statistics_reporter_svc=debug`.");
//     }

//     info!("Starting up.");

//     let host = match env::var("DB_HOST") {
//         Ok(s) => s,
//         Err(var_error) => bail!("{}: DB_HOST", var_error),
//     };
//     debug!("DB_HOST configuration loaded from process environment.");

//     let port = match env::var("DB_PORT") {
//         Ok(s) => s,
//         Err(var_error) => bail!("{}: DB_PORT", var_error),
//     };
//     debug!("DB_PORT configuration loaded from process environment.");

//     let user = match env::var("DB_USER") {
//         Ok(s) => s,
//         Err(var_error) => bail!("{}: DB_USER", var_error),
//     };
//     debug!("DB_USER configuration loaded from process environment.");

//     let pass = match env::var("DB_PASSWORD") {
//         Ok(s) => s,
//         Err(var_error) => bail!("{}: DB_PASSWORD", var_error),
//     };
//     debug!("DB_PASSWORD configuration loaded from process environment.");

//     debug!("Connecting to SQL Server `{},{}`.", host, port);

//     // Using the builder method to construct the options.
//     let mut config = Config::new();

//     config.host("10.0.0.140");
//     config.port(1433);

//     // Using SQL Server authentication.
//     config.authentication(AuthMethod::sql_server("SA", "Minifarm53tr10"));

//     // Taking the address from the configuration, using async-std's
//     // TcpStream to connect to the server.
//     let tcp = TcpStream::connect(config.get_addr()).await?;

//     // We'll disable the Nagle algorithm. Buffering is handled
//     // internally with a `Sink`.
//     tcp.set_nodelay(true)?;

//     // Handling TLS, login and other details related to the SQL Server.
//     let mut client = Client::connect(config, tcp).await?;

//     // A response to a query is a stream of data, that must be
//     // polled to the end before querying again. Using streams allows
//     // fetching data in an asynchronous manner, if needed.
//     let mut stream = client.query("SELECT @P1", &[&-4i32]).await?;

//     // As long as the `next_resultset` returns true, the stream has
//     // more results and can be polled. For each result set, the stream
//     // returns rows until the end of that result. In a case where
//     // `next_resultset` is true, polling again will return rows from
//     // the next query.
//     assert!(stream.next_resultset());

//     // In this case, we know we have only one query, returning one row
//     // and one column, so calling `into_row` will consume the stream
//     // and return us the first row of the first result.
//     let row = stream.into_row().await?;

//     assert_eq!(Some(-4i32), row.unwrap().get(0));

//     info!("Shutting down.");
//     Ok(())
// }
