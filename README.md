# conditional-s3-fetch

## Example

```rust,no_run
# use std::time::Duration;
# fn client() -> aws_sdk_s3::Client { unimplemented!() }
# async fn sleep(duration: Duration) { unimplemented!() }
# async {
# let s3_client = client();
use conditional_s3_fetch::{File, ParseString};

let mut file = File::<ParseString>::unloaded("my-bucket", "/my/path.txt");

for x in 1..10 {
    match file.fetch_data(&s3_client).await {
        Ok(Some(new)) => file = new,
        Ok(None) => println!("No modification"),
        Err(e) => eprintln!("Error: {}", e),
    }
    println!("Scheduling another update soon");
    sleep(Duration::from_secs(10)).await;
}
# };
```
