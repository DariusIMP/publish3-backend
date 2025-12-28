use actix_multipart::form::tempfile::TempFile;
use aws_config::Region;
use aws_sdk_s3::{
    Client,
    config::Credentials,
    operation::create_bucket::CreateBucketOutput,
    presigning::PresigningConfig,
    primitives::ByteStream,
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};
use base64::{Engine, engine::general_purpose};
use std::{path::PathBuf, time::Duration};

use crate::{
    common::zresult::{ZError, ZResult},
    db::s3::{S3Bucket, S3Key},
};

#[derive(Clone)]
pub struct S3Client {
    client: Client,
    region: Option<String>,
}

impl S3Client {
    pub async fn new(
        credentials: Credentials,
        region: Option<String>,
        endpoint: Option<String>,
    ) -> Self {
        let mut config_loader =
            aws_config::ConfigLoader::default().credentials_provider(credentials);

        config_loader = match region {
            Some(ref region) => config_loader.region(Region::new(region.to_owned())),
            None => {
                tracing::debug!("Region not specified. Setting 'us-east-1' region by default...");
                config_loader.region(Region::new("us-east-1"))
            }
        };

        if let Some(endpoint) = endpoint {
            config_loader = config_loader.endpoint_url(endpoint)
        }

        let sdk_config = &config_loader.load().await;
        let config = aws_sdk_s3::config::Builder::from(sdk_config).force_path_style(true);

        let client = Client::from_conf(config.build());

        S3Client { client, region }
    }

    pub async fn store_file(&self, file: &TempFile, path: Option<PathBuf>) -> ZResult<()> {
        let mut key = PathBuf::new();
        if let Some(path) = &path {
            key.push(path.to_str().unwrap());
        }

        let file_name = file
            .file_name
            .clone()
            .ok_or(ZError::from("File missing file name."))?;
        key.push(file_name);

        let key = S3Key(key.to_string_lossy().to_string());
        let body = ByteStream::read_from()
            .path(file.file.path())
            .build()
            .await
            .map_err(|e| ZError::from(format!("Failed to read file: {e}")))?;

        let mut request = self
            .client
            .put_object()
            .bucket(S3Bucket::Storage.as_str())
            .key(key.0.as_str())
            .body(body);

        if let Some(mime) = &file.content_type {
            request = request.content_type(mime.as_ref());
        }

        request.send().await?;
        Ok(())
    }

    pub async fn delete_file(&self, file_key: S3Key) -> ZResult<()> {
        tracing::debug!("Deleting file '{}'.", file_key);
        self.client
            .delete_object()
            .key(file_key.0)
            .bucket(S3Bucket::Storage.as_str())
            .customize()
            .mutate_request(|http_request| {
                if let Some(bytes) = http_request.body().bytes() {
                    let md5 = md5::compute(bytes);
                    let checksum_value = general_purpose::STANDARD.encode(md5.as_slice());
                    http_request
                        .headers_mut()
                        .append("Content-MD5", checksum_value);
                }
            })
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_file_url(&self, key: &str, bucket: &S3Bucket) -> ZResult<String> {
        let presigned_request = self
            .client
            .get_object()
            .bucket(bucket.as_str())
            .key(key.to_string())
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_secs(60 * 5))
                    .build()
                    .unwrap(),
            )
            .await?;
        Ok(presigned_request.uri().to_string())
    }

    pub async fn create_bucket(
        &self,
        bucket: S3Bucket,
        reuse_bucket: bool,
    ) -> ZResult<Option<CreateBucketOutput>> {
        let constraint = self
            .region
            .as_ref()
            .map(|region| BucketLocationConstraint::from(region.as_str()));
        let cfg = CreateBucketConfiguration::builder()
            .set_location_constraint(constraint)
            .build();
        let result = self
            .client
            .create_bucket()
            .create_bucket_configuration(cfg)
            .bucket(bucket.as_str())
            .send()
            .await;

        match result {
                Ok(output) => Ok(Some(output)),
                Err(err) => {
                    match err.into_service_error() {
                        aws_sdk_s3::operation::create_bucket::CreateBucketError::BucketAlreadyOwnedByYou(_) => {
                            if reuse_bucket {
                                Ok(None)
                            } else {
                                Err(ZError::from(
                                    "Attempted to create bucket but '{self}' but it already exists and is
                                    already owned by you while 'reuse_bucket' is set to false in the configuration."
                                ))
                            }
                        },
                        err => Err(ZError::from(format!(
                            "Couldn't create or associate bucket '{:?}': {err:?}."
                        , bucket))),
                    }
                }
            }
    }
}

#[derive(Debug)]
pub struct FileResponse {
    pub body: aws_sdk_s3::primitives::ByteStream,
    pub content_type: Option<String>,
    pub content_length: Option<String>,
    pub content_disposition: Option<String>,
    pub last_modified: Option<String>,
}
