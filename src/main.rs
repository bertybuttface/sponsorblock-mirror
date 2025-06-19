use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web_prom::PrometheusMetricsBuilder;
use once_cell::sync::Lazy;
use sqlx::PgPool;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{info, debug, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use structs::{Segment, Sponsor};

use crate::routes::{fake_is_user_vip, fake_user_info, skip_segments, skip_segments_by_id, health_check, ApiDoc};
use crate::config::Config;

mod config;
mod models;
mod routes;
mod structs;

async fn run_migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");
}


static LAST_UPDATE: Lazy<Arc<Mutex<SystemTime>>> =
    Lazy::new(|| Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    debug!("Database connection string: {}", config.database_url);
    debug!("Server will bind to: {}", config.server_bind_address());

    // Create database connection pool
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    run_migrations(&pool).await;

    // Start background task
    let pool_clone = pool.clone();
    let config_clone = config.clone();
    tokio::spawn(async move {
        background_database_task(pool_clone, config_clone).await;
    });

    info!("Starting server on {}", config.server_bind_address());

    // Create Prometheus metrics
    let prometheus = PrometheusMetricsBuilder::new(&config.metrics_namespace)
        .endpoint("/metrics")
        .build()
        .unwrap();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(prometheus.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi())
            )
            .route("/health", web::get().to(health_check))
            .route("/api/skipSegments/{hash}", web::get().to(skip_segments))
            .route("/api/skipSegments", web::get().to(skip_segments_by_id))
            .route("/api/isUserVIP", web::get().to(fake_is_user_vip))
            .route("/api/userInfo", web::get().to(fake_user_info))
    })
    .bind(config.server_bind_address())?
    .run()
    .await
}

async fn background_database_task(pool: PgPool, config: Config) {
    let mut interval = interval(config.check_interval());
    let path = Path::new(&config.csv_path);

    loop {
        interval.tick().await;
        let mut lock_guard = LAST_UPDATE.lock().await;
        let locked_last_updated_time = &mut *lock_guard;

        // see if file exists
        if path.exists() && (*locked_last_updated_time == UNIX_EPOCH || locked_last_updated_time.elapsed().unwrap_or_default() > config.file_check_interval()) {

            // Check last modified time
            let last_modified = path.metadata().unwrap().modified().unwrap();

            // Check if file was modified since last update
            if *locked_last_updated_time == UNIX_EPOCH || last_modified > *locked_last_updated_time {

                // Use COPY FROM to import the CSV file
                let start = Instant::now();
                info!("Importing database...");
                
                let mut transaction = match pool.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        error!("Failed to start transaction: {}", e);
                        continue;
                    }
                };

                let drop_temp = sqlx::query(r#"DROP TABLE IF EXISTS "sponsorTimesTemp""#)
                    .execute(&mut *transaction)
                    .await;
                
                let create_temp = sqlx::query(r#"CREATE UNLOGGED TABLE "sponsorTimesTemp"(LIKE "sponsorTimes" INCLUDING defaults INCLUDING constraints INCLUDING indexes)"#)
                    .execute(&mut *transaction)
                    .await;
                
                let copy_data = sqlx::query(&format!(r#"COPY "sponsorTimesTemp" FROM '{}' DELIMITER ',' CSV HEADER"#, path.display()))
                    .execute(&mut *transaction)
                    .await;
                
                let drop_original = sqlx::query(r#"DROP TABLE "sponsorTimes""#)
                    .execute(&mut *transaction)
                    .await;
                
                let rename_temp = sqlx::query(r#"ALTER TABLE "sponsorTimesTemp" RENAME TO "sponsorTimes""#)
                    .execute(&mut *transaction)
                    .await;
                
                let result = drop_temp.and(create_temp).and(copy_data).and(drop_original).and(rename_temp);

                match result {
                    Ok(_) => {
                        if let Err(e) = transaction.commit().await {
                            error!("Failed to commit transaction: {}", e);
                            continue;
                        }
                        info!("Imported database in {}ms", start.elapsed().as_millis());
                        
                        // Vacuum the database
                        if let Err(e) = sqlx::query(r#"VACUUM "sponsorTimes""#).execute(&pool).await {
                            error!("Failed to vacuum database: {}", e);
                        }
                        
                        *locked_last_updated_time = last_modified;
                    }
                    Err(e) => {
                        error!("Failed to import database: {}", e);
                        if let Err(rollback_err) = transaction.rollback().await {
                            error!("Failed to rollback transaction: {}", rollback_err);
                        }
                    }
                }
            }

            sleep(config.file_check_interval());
        }
    }
}
