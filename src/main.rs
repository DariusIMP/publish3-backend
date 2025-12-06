use std::sync::Arc;

use crate::{
    config::Config,
    db::{
        s3::{S3Bucket, client::S3Client},
        sql::SqlClient,
    },
};
use actix_cors::Cors;
use actix_session::{SessionMiddleware, storage::RedisSessionStore};
use actix_web::{
    App, HttpServer,
    cookie::{Key, SameSite},
    http::header,
    middleware, web,
};
use aws_sdk_s3::config::Credentials;
use dotenv::dotenv;
use lazy_static::lazy_static;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

pub mod api;
pub mod common;
pub mod config;
pub mod db;

pub struct AppState {
    sql_client: Arc<SqlClient>,
    redis_client: Client,
    s3_client: Arc<S3Client>,
}

lazy_static! {
    pub static ref CONFIG: Config = Config::init();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&CONFIG.database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {}", err);
            std::process::exit(1);
        }
    };

    let sql_client = Arc::new(SqlClient::new(pool).await);

    let redis_client = match Client::open(CONFIG.redis_url.to_owned()) {
        Ok(client) => {
            println!("✅Connection to the redis is successful!");
            client
        }
        Err(e) => {
            println!("🔥 Error connecting to Redis: {}", e);
            std::process::exit(1);
        }
    };

    let s3_credentials = Credentials::new(
        CONFIG.s3_access_key.to_owned(),
        CONFIG.s3_secret_key.to_owned(),
        None,
        None,
        "Publish3",
    );

    let s3_client =
        Arc::new(S3Client::new(s3_credentials, None, Some(CONFIG.s3_endpoint.to_owned())).await);

    s3_client
        .create_bucket(S3Bucket::Storage, true)
        .await
        .unwrap();
    
    tracing::info!("setting up Redis session storage");
    let storage = RedisSessionStore::new(CONFIG.redis_url.to_owned())
        .await
        .unwrap();

    let address = format!("{}:{}", CONFIG.server_address, CONFIG.server_port);

    tracing::info!("starting HTTP server at http://{address}");
    HttpServer::new(move || {
        let key = Key::from(CONFIG.access_token_private_key.as_bytes());
        App::new()
            .app_data(web::Data::new(AppState {
                sql_client: sql_client.clone(),
                redis_client: redis_client.clone(),
                s3_client: s3_client.clone(),
            }))
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(
                SessionMiddleware::builder(storage.clone(), key)
                    .cookie_http_only(true)
                    .cookie_same_site(SameSite::None)
                    .cookie_secure(true)
                    .build(),
            )
            .wrap(
                Cors::default()
                    .allowed_origin(&CONFIG.client_origin)
                    .allowed_methods(vec!["GET", "POST", "DELETE", "PUT"])
                    .allowed_headers(vec![
                        header::CONTENT_TYPE,
                        header::AUTHORIZATION,
                        header::ACCEPT,
                    ])
                    .supports_credentials(),
            )
        // .configure(authentication::config)
        // .configure(user::config)
        // .configure(files::config)
        // .configure(clients::config)
        // .configure(forms::config)
        // .configure(kanban::config)
        // .configure(procedures::config)
        // .configure(tasks::config)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
