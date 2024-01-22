// use bytes::Bytes;

// #[derive(Debug)]
// pub struct ParseCbor<T>(std::marker::PhantomData<T>);

// impl<T> crate::Parse for ParseCbor<T>
// where
//     T: serde::de::DeserializeOwned + serde_edn::edn_de::EDNDeserializeOwned,
// {
//     type Output = T;

//     fn parse(bytes: Bytes) -> crate::BoxedResult<Self::Output> {
//         Ok(serde_edn::from_slice(&bytes)?)
//     }
// }
