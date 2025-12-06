use actix_multipart::form::tempfile::TempFile;
use actix_web::mime::Mime;
use aws_config::Region;
use aws_sdk_s3::{
    Client,
    config::Credentials,
    operation::{
        create_bucket::CreateBucketOutput, delete_objects::DeleteObjectsOutput,
        get_object::GetObjectOutput, list_objects_v2::ListObjectsV2Output,
        put_object::PutObjectOutput,
    },
    presigning::{PresignedRequest, PresigningConfig},
    primitives::ByteStream,
    types::{
        BucketLocationConstraint, CreateBucketConfiguration, Delete, Object, ObjectIdentifier,
    },
};
use base64::{Engine, engine::general_purpose};
use std::io::Write;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tempfile::NamedTempFile;

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

    pub async fn upload_storage_files(
        &self,
        files: Vec<TempFile>,
        path: Option<PathBuf>,
    ) -> ZResult<()> {
        self.upload_files(files, path, None, &S3Bucket::Storage)
            .await
    }

    pub async fn delete_storage_files(
        &self,
        files: Vec<String>,
        path: Option<PathBuf>,
    ) -> ZResult<()> {
        self.delete_files(files, path, None, &S3Bucket::Storage)
            .await
    }

    pub async fn retrieve_storage_file(&self, path: &str) -> ZResult<FileResponse> {
        self.retrieve_file(path, &S3Bucket::Storage).await
    }

    pub async fn get_file_bytes(&self, key: &str, bucket: &S3Bucket) -> ZResult<bytes::Bytes> {
        Ok(self
            .get_object(key, bucket)
            .await?
            .body
            .collect()
            .await
            .map(|data| data.into_bytes())?)
    }

    pub async fn get_file_url(&self, key: &str, bucket: &S3Bucket) -> ZResult<String> {
        let presigned_request = self.get_object_presigned(key, bucket).await?;
        Ok(presigned_request.uri().to_string())
    }

    /// Asynchronously creates the bucket associated to this client upon construction on a new
    /// tokio runtime.
    /// Returns:
    /// - Ok(Some(CreateBucketOutput)) in case the bucket was successfully created
    /// - Ok(Some(None)) in case the `reuse_bucket` parameter is true and the bucket already exists
    ///     and is owned by you
    /// - Error in any other case
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

    async fn retrieve_file_as_tempfile(
        &self,
        key: &S3Key,
        bucket: &S3Bucket,
    ) -> ZResult<NamedTempFile> {
        let mut tempfile = NamedTempFile::new().unwrap();

        let object_output = self
            .get_object(key.0.as_str(), bucket)
            .await
            .map_err(|err| ZError::from(format!("Error retrieving file from S3: {err}")))?;

        let bytes = object_output
            .body
            .collect()
            .await
            .map_err(|err| ZError::from(format!("Failed to read file content: {err}")))?
            .into_bytes();

        tempfile.write_all(&bytes)?;

        Ok(tempfile)
    }

    async fn upload_files(
        &self,
        files: Vec<TempFile>,
        path: Option<PathBuf>,
        root_path: Option<PathBuf>,
        bucket: &S3Bucket,
    ) -> ZResult<()> {
        let upload_futures = files.into_iter().map(|temp_file| {
            let mut key = root_path.clone().unwrap_or_default();
            if let Some(path) = &path {
                key.push(path.to_str().unwrap());
            }
            key.push(temp_file.file_name.clone().unwrap());
            let key = S3Key(key.to_string_lossy().into_owned());
            async move {
                self.put_file(&key, bucket, temp_file.file, temp_file.content_type)
                    .await
            }
        });

        futures::future::try_join_all(upload_futures)
            .await
            .map_err(|err| ZError::from(format!("Error uploading files to S3: {err}")))?;

        Ok(())
    }

    async fn delete_files(
        &self,
        files: Vec<String>,
        path: Option<PathBuf>,
        root_path: Option<PathBuf>,
        bucket: &S3Bucket,
    ) -> ZResult<()> {
        let root_path = root_path.unwrap_or_default();
        let items = match path {
            Some(path) => files
                .iter()
                .map(|item| {
                    root_path
                        .join(&path)
                        .join(item)
                        .to_string_lossy()
                        .into_owned()
                })
                .collect(),
            None => files,
        };

        let results = self
            .delete_items(items, bucket)
            .await
            .map_err(|err| ZError::from(format!("Error deleting files from S3: {err}")))?;

        for result in results {
            if let Some(errors) = result.errors {
                for error in errors {
                    tracing::error!(
                        "Error deleting object from S3: key={:?}, code={:?}, message={:?}",
                        error.key,
                        error.code,
                        error.message
                    );
                }
            }
        }
        Ok(())
    }

    async fn retrieve_file(&self, key: &str, bucket: &S3Bucket) -> ZResult<FileResponse> {
        let object_output = self
            .get_object(key, bucket)
            .await
            .map_err(|err| ZError::from(format!("Error retrieving file from S3: {err}")))?;

        let content_type = object_output.content_type().map(|s| s.to_string());
        let content_length = object_output.content_length().map(|l| l.to_string());
        let content_disposition = object_output.content_disposition().map(|s| s.to_string());
        let last_modified = object_output.last_modified().map(|dt| dt.to_string());
        let body = object_output.body;

        Ok(FileResponse {
            body,
            content_type,
            content_length,
            content_disposition,
            last_modified,
        })
    }

    /// Retrieves the object associated to the [key] specified.
    async fn get_object(&self, key: &str, bucket: &S3Bucket) -> ZResult<GetObjectOutput> {
        Ok(self
            .client
            .get_object()
            .bucket(bucket.as_str())
            .key(key.to_string())
            .send()
            .await?)
    }

    /// Retrieves the object associated to the [key] specified.
    async fn get_object_presigned(
        &self,
        key: &str,
        bucket: &S3Bucket,
    ) -> ZResult<PresignedRequest> {
        Ok(self
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
            .await?)
    }

    async fn put_file(
        &self,
        key: &S3Key,
        bucket: &S3Bucket,
        file: NamedTempFile,
        content_type: Option<Mime>,
    ) -> ZResult<PutObjectOutput> {
        let body = ByteStream::read_from()
            .path(file.path())
            .build()
            .await
            .map_err(|e| ZError::from(format!("Failed to read file: {e}")))?;

        let mut request = self
            .client
            .put_object()
            .bucket(bucket.as_str())
            .key(key.0.as_str())
            .body(body);

        if let Some(mime) = content_type {
            request = request.content_type(mime.as_ref());
        }

        Ok(request.send().await?)
    }

    async fn delete_items(
        &self,
        items: Vec<String>,
        bucket: &S3Bucket,
    ) -> ZResult<Vec<DeleteObjectsOutput>> {
        let deletions = items
            .into_iter()
            .map(async |item| -> ZResult<DeleteObjectsOutput> {
                let objects = self.list_objects_with_prefix(&item, bucket).await?;
                let result = self.delete_objects(objects, bucket).await;
                tracing::debug!("Deleted item '{}'.", item);
                result
            });

        futures::future::try_join_all(deletions).await
    }

    async fn delete_objects(
        &self,
        objects: Vec<Object>,
        bucket: &S3Bucket,
    ) -> ZResult<DeleteObjectsOutput> {
        if objects.is_empty() {
            return Ok(DeleteObjectsOutput::builder()
                .set_deleted(Some(vec![]))
                .build());
        }

        let mut object_identifiers: Vec<ObjectIdentifier> = vec![];

        for object in objects {
            let identifier = ObjectIdentifier::builder()
                .set_key(object.key().map(|x| x.to_string()))
                .build()?;
            object_identifiers.push(identifier);
        }

        let delete = Delete::builder()
            .set_objects(Some(object_identifiers))
            .build()?;

        Ok(self
            .client
            .delete_objects()
            .bucket(bucket.as_str())
            .delete(delete)
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
            .await?)
    }

    async fn list_objects_with_prefix(
        &self,
        prefix: &String,
        bucket: &S3Bucket,
    ) -> ZResult<Vec<Object>> {
        let response = self
            .client
            .list_objects_v2()
            .bucket(bucket.as_str())
            .set_prefix(Some(prefix.to_owned()))
            .send()
            .await?;
        Ok(response.contents().to_vec())
    }

    pub async fn list_objects_in_directory(
        &self,
        bucket: &S3Bucket,
        directory_path: &Path,
    ) -> ZResult<ListObjectsV2Output> {
        Ok(self
            .client
            .list_objects_v2()
            .bucket(bucket.as_str())
            .set_prefix(Some(directory_path.to_string_lossy().into_owned()))
            .delimiter("/")
            .send()
            .await?)
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
