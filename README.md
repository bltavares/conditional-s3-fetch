# conditional-s3-fetch
File container struct that fetches and parses S3 files, with conditional fetching.

<hr />



## Example

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
