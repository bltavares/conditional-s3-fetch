//! CBOR (Concise Binary Object Representation) parser (feature: `cbor`)
//!
//! Parser implementation to read CBOR data into a deserialized object.
//!
//! # Example
//!
//! ```rust
//! # #[derive(serde::Deserialize)]
//! # struct MyStruct;
//! use conditional_s3_fetch::{File, Cbor};
//!
//! let file = File::<Cbor<MyStruct>>::unloaded("bucket", "/data/key.cbor");
//! ```
use bytes::Bytes;

/// Parser implementation to read CBOR data into a deserialized object.
///
/// # Example
///
///  ```rust
/// # #[derive(serde::Deserialize)]
/// # struct MyStruct;
/// use conditional_s3_fetch::{File, Cbor};
///
/// let file = File::<Cbor<MyStruct>>::unloaded("bucket", "/data/key.cbor");
/// ```
#[derive(Debug)]
pub struct Cbor<T>(std::marker::PhantomData<T>);

impl<T> crate::Parse for Cbor<T>
where
    T: serde::de::DeserializeOwned,
{
    type Output = T;

    fn parse(bytes: Bytes) -> crate::BoxedResult<Self::Output> {
        Ok(cbor4ii::serde::from_slice(&bytes)?)
    }
}
