use bytes::{Buf, Bytes};

#[cfg(all(feature = "json", feature = "simd-json"))]
compile_error!("Cannot enable both json and simd-json features");

#[derive(Debug)]
pub struct ParseJson<T>(std::marker::PhantomData<T>);

impl<T> crate::Parse for ParseJson<T>
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
