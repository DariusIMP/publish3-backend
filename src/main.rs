use std::sync::Arc;

use crate::{
    blockchain::CapabilitySigner,
    config::Config,
    db::{
        s3::{S3Bucket, client::S3Client},
        sql::SqlClient,
    },
};
use actix_cors::Cors;
use actix_web::{App, HttpServer, http::header, middleware, web};
use aptos_rust_sdk::client::config::AptosNetwork;
use aptos_rust_sdk::client::{builder::AptosClientBuilder, rest_api::AptosFullnodeClient};
use dotenv::dotenv;
use lazy_static::lazy_static;
use privy_rs::PrivyClient;
use sqlx::postgres::PgPoolOptions;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use url::Url;

pub mod api;
pub mod auth;
pub mod blockchain;
pub mod common;
pub mod config;
pub mod db;

pub struct AppState {
    sql_client: Arc<SqlClient>,
    s3_client: Arc<S3Client>,
    aptos_client: Arc<AptosFullnodeClient>,
    privy_client: Arc<PrivyClient>,
}

lazy_static! {
    pub static ref CONFIG: Config = Config::init();
    pub static ref CAPABILITY_SIGNER: CapabilitySigner =
        CapabilitySigner::from_config(&CONFIG).unwrap();
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

    let privy_client = Arc::new(PrivyClient::new_from_env().unwrap());
    let builder = AptosClientBuilder::new(AptosNetwork::new(
        &CONFIG.movement_network,
        Url::parse(CONFIG.movement_rpc_url.as_str()).unwrap(),
        Url::parse(CONFIG.movement_indexer_url.as_str()).unwrap(),
    ));

    let aptos_client = Arc::new(builder.build());

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

    let s3_client = create_s3_client().await;

    s3_client
        .create_bucket(S3Bucket::Storage, true)
        .await
        .unwrap();

    let address = format!("{}:{}", CONFIG.server_address, CONFIG.server_port);

    tracing::info!("starting HTTP server at http://{address}");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                sql_client: sql_client.clone(),
                s3_client: s3_client.clone(),
                aptos_client: aptos_client.clone(),
                privy_client: privy_client.clone(),
            }))
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(crate::auth::Privy)
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
            .configure(api::config)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

#[cfg(not(feature = "aws-s3"))]
async fn create_s3_client() -> Arc<S3Client> {
    use aws_sdk_s3::config::Credentials;
    let s3_credentials = Credentials::new(
        CONFIG.s3_access_key.to_owned(),
        CONFIG.s3_secret_key.to_owned(),
        None,
        None,
        "Publish3",
    );
    Arc::new(S3Client::new(s3_credentials, None, Some(CONFIG.s3_endpoint.to_owned())).await)
}

#[cfg(feature = "aws-s3")]
async fn create_s3_client() -> Arc<S3Client> {
    Arc::new(S3Client::new_from_env().await)
}
