pub mod client;

use crate::db::s3::client::S3Client;

#[derive(Debug, Clone, Copy)]
pub enum S3Bucket {
    Storage,
}

impl S3Bucket {
    pub const STORAGE: &'static str = "storage";

    pub fn as_str(&self) -> &'static str {
        match self {
            S3Bucket::Storage => Self::STORAGE,
        }
    }
}

impl From<S3Bucket> for &'static str {
    fn from(bucket: S3Bucket) -> Self {
        bucket.as_str()
    }
}

pub trait S3Contents {
    fn load_s3_contents(&self, s3_client: &S3Client) -> impl Future<Output = Self>;
}

#[derive(Debug, Clone)]
pub struct S3Key(pub String);

impl From<S3Key> for String {
    fn from(s3_key: S3Key) -> Self {
        s3_key.0
    }
}

impl std::fmt::Display for S3Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
