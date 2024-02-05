//! Json parser implementation. (feature `json` or `simd-json`)
//!
//! Parser implementation to read Json data into a deserialized object.
//!
//! # Example
//!
//! ```rust
//! # #[derive(serde::Deserialize)]
//! # struct MyStruct;
//! use conditional_s3_fetch::{File, Json};
//!
//! let file = File::<Json<MyStruct>>::unloaded("bucket", "/data/key.Json");
//! ```
use bytes::{Buf, Bytes};

#[cfg(all(feature = "json", feature = "simd-json"))]
compile_error!("Cannot enable both json and simd-json features");

/// Parser implementation to read Json data into a deserialized object.
///
/// # Example
///
///  ```rust
/// # #[derive(serde::Deserialize)]
/// # struct MyStruct;
/// use conditional_s3_fetch::{File, Json};
///
/// let file = File::<Json<MyStruct>>::unloaded("bucket", "/data/key.Json");
/// ```
#[derive(Debug)]
pub struct Json<T>(std::marker::PhantomData<T>);

impl<T> crate::Parse for Json<T>
where
    T: serde::de::DeserializeOwned,
{
    type Output = T;

    #[cfg(feature = "json")]
    fn parse(bytes: Bytes) -> crate::BoxedResult<Self::Output> {
        let buffer = bytes.reader();
        Ok(serde_json::from_reader(buffer)?)
    }

    #[cfg(feature = "simd-json")]
    fn parse(bytes: Bytes) -> crate::BoxedResult<Self::Output> {
        let buffer = bytes.reader();
        Ok(simd_json::from_reader(buffer)?)
    }
}
