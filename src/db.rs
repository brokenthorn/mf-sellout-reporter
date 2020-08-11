//! # DB
//!
//! Provides database connections.
//! Currently there is not connection pooling built in.

use async_std::net::TcpStream;
use tiberius::{AuthMethod, Client, Config};
use anyhow::Result;

/// MSSQL connections object.
#[derive(Debug, Clone)]
pub struct MSSQL;

/// A `tiberius` MSSQL client running on `async_std`'s `TcpStream`;
pub type TiberiusClient = Client<TcpStream>;

impl MSSQL {
    /// Connects to a MSSQL database server (configured via environment variables)
    /// and returns a `tiberius::Client` for that server.
    pub async fn connect(&self) -> Result<TiberiusClient> {
        let mssql_host = std::env::var("MSSQL_HOST").or(Err(Error::from_str(StatusCode::InternalServerError, "Failed to get MSSQL_HOST")))?;
        let mssql_port = std::env::var("MSSQL_PORT").or(Err(Error::from_str(
            StatusCode::InternalServerError,
            "Failed to get MSSQL_PORT",
        )))?;
        let mssql_user = std::env::var("MSSQL_USER").or(Err(Error::from_str(
            StatusCode::InternalServerError,
            "Failed to get MSSQL_USER",
        )))?;
        let mssql_password = std::env::var("MSSQL_PASSWORD").or(Err(Error::from_str(
            StatusCode::InternalServerError,
            "Failed to get MSSQL_PASSWORD",
        )))?;

        tide::log::info!("Connecting to MSSQL Server: {}:{}.", mssql_host, mssql_port);

        let mut config = Config::new();
        config.host(mssql_host.clone());
        config.port(mssql_port.parse::<u16>().or(Err(Error::from_str(
            StatusCode::InternalServerError,
            "Failed to parse MSSQL_PORT as u16",
        )))?);
        config.authentication(AuthMethod::sql_server(mssql_user, mssql_password));
        let tcp_stream = TcpStream::connect(config.get_addr()).await?;
        // We'll disable the Nagle algorithm. Buffering is handled internally with a `Sink`.
        tcp_stream.set_nodelay(true)?;
        let client = Client::connect(config, tcp_stream).await?;

        tide::log::info!("Connected to MSSQL Server: {}:{}", mssql_host, mssql_port);
        Ok(client)
    }

    /// Calls `[BizPharmaHO].[dbo].[spBPWSWebGetStoc]` stored procedure and returns the results as
    /// [`ApiResponse<Stock>`](ApiResponse).
    pub async fn sp_bpws_web_get_stoc(
        &self,
        data_ultima_actualizare: &str,
        data_curenta: &str,
    ) -> Result<ApiResponse<Stock, String>> {
        // REF: Consider using futures combinators and rewrite without variable declarations.
        let mut client = self.connect().await?;
        let query_result = client.query(
            "DECLARE @mesaj_eroare VARCHAR(255); EXEC [BizPharmaHO].[dbo].[spBPWSWebGetStoc] @DataUltimaActualizare = @P1, @DataCurenta = @P2, @MesajEroare = @mesaj_eroare OUTPUT; SELECT @mesaj_eroare AS [mesaj_eroare];",
            &[&data_ultima_actualizare, &data_curenta],
        ).await?;
        let result = stock_query_result_to_api_response(query_result).await?;
        Ok(result)
    }
}
