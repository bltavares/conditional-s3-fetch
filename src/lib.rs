#![doc = include_str!("../README.md")]

use aws_sdk_s3::{error::SdkError, operation::get_object::GetObjectOutput};
use bytes::Bytes;
use std::{marker::PhantomData, ops::Deref};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("S3 Access Error: {0}")]
    SdkError(#[from] Box<dyn std::error::Error + Send>),
    #[error("Read Error: {0}")]
    ReadError(#[from] aws_sdk_s3::primitives::ByteStreamError),
    #[error("Parse Error: {0}")]
    ParseError(Box<dyn std::error::Error + Send>),
}

type Result<T> = std::result::Result<T, Error>;

#[cfg(any(feature = "json", feature = "simd-json"))]
pub mod json;
#[cfg(any(feature = "json", feature = "simd-json"))]
pub use json::ParseJson;

#[cfg(feature = "cbor")]
pub mod cbor;
#[cfg(feature = "cbor")]
pub use cbor::ParseCbor;

// Needs https://github.com/alex-dixon/serde_edn published on crates.io
// Meanwhile use the git dependency and implement the trait on your project
// #[cfg(feature = "edn")]
// pub mod edn;
// #[cfg(feature = "edn")]
// pub use edn::ParseEdn;

#[derive(Debug)]
pub struct Content<T> {
    etag: String,
    body: T,
}

impl<T> Content<T> {
    pub fn into_inner(self) -> T {
        self.body
    }
}

impl<T> Deref for Content<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

#[derive(Debug)]
pub enum File<P>
where
    P: Parse,
{
    Unloaded {
        bucket: String,
        path: String,
        _mark: PhantomData<P>,
    },
    Loaded {
        bucket: String,
        path: String,
        inner: Content<P::Output>,
        _mark: PhantomData<P>,
    },
}

impl<P> File<P>
where
    P: Parse,
{
    fn path(&self) -> &str {
        match self {
            Self::Unloaded { path, .. } | Self::Loaded { path, .. } => path,
        }
    }

    fn bucket(&self) -> &str {
        match self {
            Self::Unloaded { bucket, .. } | Self::Loaded { bucket, .. } => bucket,
        }
    }

    pub fn as_content(&self) -> Option<&Content<P::Output>> {
        match self {
            Self::Unloaded { .. } => None,
            Self::Loaded { inner, .. } => Some(inner),
        }
    }
}

type BoxedResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub trait Parse {
    type Output;
    fn parse(bytes: bytes::Bytes) -> BoxedResult<Self::Output>;
}

#[derive(Debug)]
pub struct ParseString;
impl Parse for ParseString {
    type Output = String;
    fn parse(bytes: Bytes) -> BoxedResult<Self::Output> {
        Ok(String::from_utf8(bytes.into())?)
    }
}

#[derive(Debug)]
pub struct ParseBytes;
impl Parse for ParseBytes {
    type Output = Bytes;
    fn parse(bytes: Bytes) -> BoxedResult<Self::Output> {
        Ok(bytes)
    }
}

impl<P> File<P>
where
    P: Parse,
{
    pub fn unloaded<S: Into<String>>(bucket: S, path: S) -> Self {
        Self::Unloaded {
            bucket: bucket.into(),
            path: path.into(),
            _mark: PhantomData,
        }
    }

    async fn attempt_extract(&self, response: GetObjectOutput) -> self::Result<Self> {
        let bytes = response
            .body
            .collect()
            .await
            .map_err(Error::ReadError)?
            .into_bytes();
        let body = P::parse(bytes).map_err(|e| Error::ParseError(e))?;

        let etag = response.e_tag.unwrap_or_default();
        Ok(Self::Loaded {
            bucket: self.bucket().into(),
            path: self.path().into(),
            inner: Content { etag, body },
            _mark: PhantomData,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn fetch_data(
        &self,
        s3_client: &aws_sdk_s3::Client,
    ) -> self::Result<Option<self::File<P>>>
    where
        P: Parse,
    {
        let mut response_builder = s3_client
            .get_object()
            .bucket(self.bucket())
            .key(self.path());

        if let File::Loaded { inner, .. } = &self {
            response_builder = response_builder.if_none_match(&inner.etag);
        }

        let response = response_builder.send().await;
        let response = if let Err(SdkError::ServiceError(e)) = &response {
            if e.raw().status().as_u16() == 304 {
                return Ok(None);
            }
            response.map_err(|e| Error::SdkError(Box::new(e)))?
        } else {
            response.map_err(|e| Error::SdkError(Box::new(e)))?
        };

        Ok(Some(self.attempt_extract(response).await?))
    }
}
