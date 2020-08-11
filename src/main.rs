#![forbid(unsafe_code)]
#![forbid(deprecated_in_future)]
#![warn(missing_docs)]
// TODO: Remove these when done:
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]


//! # MF Sellout Reporter
//!
//! A CRON-like service purposefully built for periodically reporting sellout data
//! to contract research organizations (CRO's) in the health information technology industry.

// pub mod iqvia;
pub mod job;

#[macro_use]
extern crate anyhow;

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> anyhow::Result<()> {
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
