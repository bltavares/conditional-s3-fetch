use aws_sdk_s3::{Client, Config};
use conditional_s3_fetch::{Cbor, File, Json, Parse};
use futures::future::FutureExt;
use std::time::Duration;
use tokio::time::sleep;

async fn fetch_file<T>(path: &str) -> File<T>
where
    T: Parse,
{
    let conf = Config::builder()
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            "example",
            "secret_key",
            None,
            None,
            "test",
        ))
        .endpoint_url("http://127.0.0.1:9000")
        .region(aws_sdk_s3::config::Region::new("test"))
        .behavior_version_latest()
        .build();
    let s3_client = Client::from_conf(conf);

    let mut file = File::<T>::unloaded("example-bucket", path);

    for x in 1..3 {
        println!("{}x - Fetching data for {:?}", x, &file);
        match file.fetch_data(&s3_client).await {
            Ok(Some(new)) => file = new,
            Ok(None) => println!("No modification for {:?}", &file),
            Err(e) => eprintln!("Error: {:?} on {:?}", e, &file),
        }
        println!("Scheduling another update soon");
        sleep(Duration::from_secs(10)).await;
    }

    file
}

#[derive(serde::Deserialize, Debug)]
pub struct Data {
    pub message: String,
}

#[tokio::main]
async fn main() {
    let json = tokio::spawn(fetch_file::<Json<Data>>("hello.json").then(|f| async move {
        println!("Json struct {:?} with {:?}", f, f.as_content());
    }));
    let cbor = tokio::spawn(fetch_file::<Cbor<Data>>("hello.cbor").then(|f| async move {
        println!("Cbor struct {:?} with {:?}", f, f.as_content());
    }));
    let string = tokio::spawn(fetch_file::<String>("hello.txt").then(|f| async move {
        println!("String struct {:?} with {:?}", f, f.as_content());
    }));
    let bytes = tokio::spawn(
        fetch_file::<bytes::Bytes>("hello.txt").then(|f| async move {
            println!("Bytes struct {:?} with {:?}", f, f.as_content());
        }),
    );
    let vec = tokio::spawn(fetch_file::<Vec<u8>>("hello.txt").then(|f| async move {
        println!("Vec<u8> struct {:?} with {:?}", f, f.as_content());
    }));

    tokio::try_join!(json, cbor, string, bytes, vec)
        .map(drop)
        .unwrap_or_else(|e| eprintln!("Error: {:?}", e));
}
