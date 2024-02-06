# conditional-s3-fetch
File container struct that fetches and parses S3 files, with conditional fetching.

<hr />

Often you'll need to implement a background file fetcher on a longer-live process. To avoid unnecessary fetches, you can use the `conditional-s3-fetch` crate to fetch files from S3 only when they have been modified.

It will use the `If-None-Match` header with the provided AWS S3 `ETag` response header, to avoid fetching and parsing the file if it hasn't been modified.
This crate provides a `File` struct which adds metadata to the parsed content, so it can be reused in future fetch calls.

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
conditional-s3-fetch = "0.1.0"
```

Provided file parses are:
- [`String`]
- [`Vec<u8>`]
- [`bytes::Bytes`]


Additional schemaless file format parses provided on this crate:
- `simd-json` (default) or `json`: Provides the `Json` parser to help read files into structure.
- `cbor` (default): Provides the `Cbor` parser to help read files into structure.

You can customize which built-in additional parser is provided by disabling the default features and enabling the desired one.

```toml
[dependencies]
conditional-s3-fetch = { version = "0.1.0", default-features = false, features = ["json"] }
```

## Example

You can start with a [`File::unloaded`] instance which doesn't have any data, and then fetch it using the `fetch_data` method later, such as a background process loop.

```rust,ignore,text
use conditional_s3_fetch::File;;

let mut file = File::<String>::unloaded("my-bucket", "/my/path.txt");

for x in 1..10 {
    match file.fetch_data(&s3_client).await {
        Ok(Some(new)) => file = new,
        Ok(None) => println!("No modification"),
        Err(e) => eprintln!("Error: {}", e),
    }
    println!("Scheduling another update soon");
    sleep(Duration::from_secs(10)).await;
}
```

Adding shared mutable-access, such as `Arc`'s are left as an exercise to each project to better fit their needs.

## Implementing a custom parser

You can implement your own parser by implementing the [`Parse`] trait with your custom parser logic.
It then can be called with a `File::<MyParser>` turbofish syntax.

```rust
use conditional_s3_fetch::{File, Parse, BoxedResult};

struct MyParser;

impl Parse for MyParser {
    type Output = String;

    fn parse(data: bytes::Bytes) -> BoxedResult<Self::Output> {
        match data.as_ref() {
            b"hello" => Ok("world".to_string()),
            _ => Err("Invalid data".into()),
        }
    }
}

let file = File::<MyParser>::unloaded("my-bucket", "/my/path.txt");
```

## Local development

There is an example binary that can be used to test the crate locally, using a `minio` container locally.

```sh
cd example
docker-compose up -d
cargo run --example watcher
docker-compose down
```