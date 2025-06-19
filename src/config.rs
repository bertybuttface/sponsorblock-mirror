use std::env;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub log_level: String,
    pub csv_path: String,
    pub check_interval_seconds: u64,
    pub file_check_interval_seconds: u64,
    pub metrics_namespace: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable must be set".to_string())?;

        let server_host = env::var("SERVER_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8001".to_string())
            .parse::<u16>()
            .map_err(|_| "SERVER_PORT must be a valid port number".to_string())?;

        let log_level = env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "sponsorblock_mirror=debug,actix_web=info".to_string());

        let csv_path = env::var("CSV_PATH")
            .unwrap_or_else(|_| "mirror/sponsorTimes.csv".to_string());

        let check_interval_seconds = env::var("CHECK_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|_| "CHECK_INTERVAL_SECONDS must be a valid number".to_string())?;

        let file_check_interval_seconds = env::var("FILE_CHECK_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .map_err(|_| "FILE_CHECK_INTERVAL_SECONDS must be a valid number".to_string())?;

        let metrics_namespace = env::var("METRICS_NAMESPACE")
            .unwrap_or_else(|_| "api".to_string());

        Ok(Config {
            database_url,
            server_host,
            server_port,
            log_level,
            csv_path,
            check_interval_seconds,
            file_check_interval_seconds,
            metrics_namespace,
        })
    }

    pub fn server_bind_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    pub fn check_interval(&self) -> Duration {
        Duration::from_secs(self.check_interval_seconds)
    }

    pub fn file_check_interval(&self) -> Duration {
        Duration::from_secs(self.file_check_interval_seconds)
    }
}