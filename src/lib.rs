#![doc = include_str!("../README.md")]
use aws_sdk_s3::{error::SdkError, operation::get_object::GetObjectOutput};
use bytes::Bytes;
use std::{fmt, marker::PhantomData, ops::Deref};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("S3 Access Error: {0}")]
    SdkError(#[from] Box<dyn std::error::Error + Send>),
    #[error("Read Error: {0}")]
    ReadError(#[from] aws_sdk_s3::primitives::ByteStreamError),
    #[error("Parse Error: {0}")]
    ParseError(Box<dyn std::error::Error + Send>),
    #[error("Unabled to convert an unloaded file to a loaded file")]
    UnabledToLoad,
}

type Result<T> = std::result::Result<T, Error>;

#[cfg(any(feature = "json", feature = "simd-json"))]
pub mod json;
#[cfg(any(feature = "json", feature = "simd-json"))]
pub use json::Json;

#[cfg(feature = "cbor")]
pub mod cbor;
#[cfg(feature = "cbor")]
pub use cbor::Cbor;

// Needs https://github.com/alex-dixon/serde_edn published on crates.io
// Meanwhile use the git dependency and implement the trait on your project
// #[cfg(feature = "edn")]
// pub mod edn;
// #[cfg(feature = "edn")]
// pub use edn::ParseEdn;

/// Container struct to hold the parsed content and the `ETag` of the file
///
/// It implements [`Deref`] to allow using the inner `T` methods directly.
///
/// # Example
/// ```rust,no_run
/// # fn data() -> Content<String> { unimplemented!() }
/// use conditional_s3_fetch::Content;
///
/// let content: Content<String> = data();
/// assert_eq!(content.len(), 13);
/// ```
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Content<T> {
    etag: String,
    body: T,
}

impl<T> Content<T> {
    /// Converts the container struct into the inner value
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

/// Container struct to hold S3 file metadata to be fetched
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct UnloadedFile<P>
where
    P: Parse,
{
    bucket: String,
    path: String,
    parser: PhantomData<P>,
}

/// Container struct to hold S3 file metadata and parsed content
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct LoadedFile<P>
where
    P: Parse,
{
    bucket: String,
    path: String,
    inner: Content<P::Output>,
    parser: PhantomData<P>,
}

/// Container struct that holds either a reference to an unloaded file or a loaded file with it's content parsed.
///
/// Given a `P: Parse` implementation, it will parse the content of the file when it's loaded.
///
/// # Example
/// ```rust,no_run
/// # fn client() -> aws_sdk_s3::Client { unimplemented!() }
/// # #[derive(serde::Deserialize, Debug)]
/// # struct MyStruct;
/// # async {
/// # let s3_client = client();
/// use conditional_s3_fetch::{File, Json};
///
/// // Unloaded file reference parsed as String.
/// let unloaded_file = File::<String>::unloaded("my-bucket", "/my/path.txt");
///
/// // Loaded file referenced parsed as Vec<u8>.
/// let loaded_file = File::<Vec<u8>>::loaded("my-bucket", "/my/path.txt", &s3_client)
///     .await
///     .expect("Failed to load the file");
///
/// /// File to be parsed from a struct using the `Json` parser.
/// let mut json_file = File::<Json<MyStruct>>::unloaded("my-bucket", "/my/path.json");
/// let new_json_file = json_file.fetch(&s3_client)
///     .await
///     .expect("Failed to load the file");
/// if let Some(new) = new_json_file {
///     json_file = new;
/// }
///
/// println!("{:?}", json_file.as_content().as_ref());
///
/// # };
/// ```
pub enum File<P>
where
    P: Parse,
{
    /// Reference to an unloaded file on S3
    Unloaded(UnloadedFile<P>),
    /// Reference to a loaded file on S3 with parsed content
    Loaded(LoadedFile<P>),
}

impl<P> PartialEq for File<P>
where
    P: Parse + PartialEq,
    P::Output: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unloaded(l0), Self::Unloaded(r0)) => l0 == r0,
            (Self::Loaded(l0), Self::Loaded(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<P> Eq for File<P>
where
    P: Parse + PartialEq,
    P::Output: Eq,
{
}

impl<P> File<P>
where
    P: Parse,
{
    /// Returns the path of the file inside the bucket
    pub fn path(&self) -> &str {
        match self {
            Self::Unloaded(UnloadedFile { path, .. }) | Self::Loaded(LoadedFile { path, .. }) => {
                path
            }
        }
    }

    /// Returns the bucket of the file
    pub fn bucket(&self) -> &str {
        match self {
            Self::Unloaded(UnloadedFile { bucket, .. })
            | Self::Loaded(LoadedFile { bucket, .. }) => bucket,
        }
    }

    /// Return the reference to the inner content if the file is [`File::loaded`]
    pub fn as_content(&self) -> Option<&Content<P::Output>> {
        match self {
            Self::Unloaded(_) => None,
            Self::Loaded(file) => Some(&file.inner),
        }
    }

    /// Returns the inner value of the parsed struct if the file is [`File::loaded`]
    ///
    /// Drops all the metadata related to the S3 File reference.
    pub fn into_inner(self) -> Option<P::Output> {
        match self {
            Self::Unloaded { .. } => None,
            Self::Loaded(file) => Some(file.inner.into_inner()),
        }
    }
}

impl<P> fmt::Debug for File<P>
where
    P: Parse,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unloaded(UnloadedFile {
                bucket,
                path,
                parser,
                ..
            }) => {
                write!(f, "File#Unloaded<{bucket}:{path} parser={parser:?}>")
            }
            Self::Loaded(LoadedFile {
                bucket,
                path,
                inner,
                parser,
                ..
            }) => {
                write!(
                    f,
                    "File#Loaded<{bucket}:{path} parser={parser:?} etag={}>",
                    &inner.etag
                )
            }
        }
    }
}

/// Alias for boxed error handling during parsing
pub type BoxedResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Trait to parse the [`File`] content after fetching it from S3
pub trait Parse {
    type Output;

    /// Parse the file content from S3
    ///
    /// This method is called when the file is fetched from S3 and the content is ready to be parsed.
    ///
    /// # Errors
    /// Returns an [`Error`] if the content could not be parsed.
    fn parse(bytes: bytes::Bytes) -> BoxedResult<Self::Output>;
}

/// Parse a [`File`] content as a [`String`] struct
///
/// # Example
///
///  ```rust
/// use conditional_s3_fetch::File;
///
/// let file = File::<String>::unloaded("bucket", "/data/my-data.txt");
/// ```
impl Parse for String {
    type Output = Self;
    fn parse(bytes: Bytes) -> BoxedResult<Self::Output> {
        Ok(String::from_utf8(bytes.into())?)
    }
}

/// Parse a [`File`] content as a [`Bytes`] struct
///
///  # Example
///
///  ```rust
/// use bytes::Bytes;
/// use conditional_s3_fetch::File;
///
/// let file = File::<Bytes>::unloaded("bucket", "/data/data.dat");
/// ```
impl Parse for bytes::Bytes {
    type Output = Self;
    fn parse(bytes: Bytes) -> BoxedResult<Self::Output> {
        Ok(bytes)
    }
}

/// Parse a [`File`] content as a [`Vec<u8>`] struct
/// # Example
///
///  ```rust
/// use conditional_s3_fetch::File;
///
/// let file = File::<Vec<u8>>::unloaded("bucket", "/data/data.dat");
/// ```
impl Parse for Vec<u8> {
    type Output = Self;
    fn parse(bytes: Bytes) -> BoxedResult<Self::Output> {
        Ok(bytes.to_vec())
    }
}

impl<P> File<P>
where
    P: Parse,
{
    /// Creates a reference to an unloaded file on S3
    ///
    /// This file can be loaded in a later time using the `fetch` method.
    /// Useful for initialization process, as a [`Client`](aws_sdk_s3::Client) is not required or where fetching the file happens in background.
    ///
    ///  ## Example
    ///
    /// ```rust,no_run
    /// # use std::time::Duration;
    /// # fn client() -> aws_sdk_s3::Client { unimplemented!() }
    /// # async fn sleep(duration: Duration) { unimplemented!() }
    /// # async {
    /// # let s3_client = client();
    /// use conditional_s3_fetch::File;
    ///
    /// let mut file = File::<String>::unloaded("my-bucket", "/my/path.txt");
    ///
    /// for x in 1..10 {
    ///     match file.fetch(&s3_client).await {
    ///         Ok(Some(new)) => file = new,
    ///         Ok(None) => println!("No modification"),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    ///     println!("Scheduling another update soon");
    ///     sleep(Duration::from_secs(10)).await;
    /// }
    /// # };
    /// ```
    pub fn unloaded<S: Into<String>>(bucket: S, path: S) -> Self {
        Self::Unloaded(UnloadedFile {
            bucket: bucket.into(),
            path: path.into(),
            parser: PhantomData,
        })
    }

    /// Creates a reference to a loaded file on S3, already with parsed data
    ///
    /// Useful for initialization process where failure should halt the service.
    /// A [`Client`](aws_sdk_s3::Client) is **required**.
    ///
    /// This file can be refreshed using the `fetch` method.
    ///
    ///  ## Example
    ///
    /// ```rust,no_run
    /// # use std::time::Duration;
    /// # fn client() -> aws_sdk_s3::Client { unimplemented!() }
    /// # async fn sleep(duration: Duration) { unimplemented!() }
    /// # async {
    /// # let s3_client = client();
    /// use conditional_s3_fetch::File;
    ///
    /// let mut file = File::<String>::loaded("my-bucket", "/my/path.txt", &s3_client)
    ///     .await
    ///     .expect("Failed to initially fetch the file");
    ///
    /// for x in 1..10 {
    ///     match file.fetch(&s3_client).await {
    ///         Ok(Some(new)) => file = new,
    ///         Ok(None) => println!("No modification"),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    ///     println!("Scheduling another update soon");
    ///     sleep(Duration::from_secs(10)).await;
    /// }
    /// # };
    /// ```
    ///
    /// # Errors
    /// Returns an [`Error`] if the content could not be fetched or parsed.
    pub async fn loaded<S: Into<String>>(
        bucket: S,
        path: S,
        s3_client: &aws_sdk_s3::Client,
    ) -> Result<Self> {
        let file = Self::unloaded(bucket, path);
        let fetch = file.fetch(s3_client).await?;
        fetch.ok_or_else(|| Error::UnabledToLoad)
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
        Ok(Self::Loaded(LoadedFile {
            bucket: self.bucket().into(),
            path: self.path().into(),
            inner: Content { etag, body },
            parser: PhantomData,
        }))
    }

    /// Attempt to fetch the file from S3 using `If-None-Match` header
    ///
    /// If the file has not been modified, it returns `None`.
    /// If the file has been modified, returns a new [`File`] with the new content already parsed.
    /// If there are any errors during the process, returns an error of [`Error`].
    ///
    /// # Errors
    /// Returns an [`Error`] if the content could not be fetched or parsed.
    #[tracing::instrument(skip_all)]
    pub async fn fetch(&self, s3_client: &aws_sdk_s3::Client) -> self::Result<Option<self::File<P>>>
    where
        P: Parse,
    {
        let mut response_builder = s3_client
            .get_object()
            .bucket(self.bucket())
            .key(self.path());

        if let File::Loaded(LoadedFile { inner, .. }) = &self {
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
