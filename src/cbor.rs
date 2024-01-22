use bytes::Bytes;

#[derive(Debug)]
pub struct ParseCbor<T>(std::marker::PhantomData<T>);

impl<T> crate::Parse for ParseCbor<T>
where
    T: serde::de::DeserializeOwned,
{
    type Output = T;

    fn parse(bytes: Bytes) -> crate::BoxedResult<Self::Output> {
        Ok(cbor4ii::serde::from_slice(&bytes)?)
    }
}
