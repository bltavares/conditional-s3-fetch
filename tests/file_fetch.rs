use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::SdkBody,
    Client, Config,
};
use aws_smithy_runtime::client::http::test_util::{ReplayEvent, StaticReplayClient};

use conditional_s3_fetch::File;

fn test_client(replay_client: StaticReplayClient) -> Client {
    Client::from_conf(
        Config::builder()
            .behavior_version_latest()
            .credentials_provider(Credentials::new(
                "ATESTCLIENT",
                "astestsecretkey",
                Some("atestsessiontoken".to_string()),
                None,
                "",
            ))
            .region(Region::new("us-east-1"))
            .http_client(replay_client)
            .build(),
    )
}

#[tokio::test]
async fn test_fetching_file() {
    let req1 = ReplayEvent::new(
        http::Request::builder()
            .method("GET")
            .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
            .body(SdkBody::empty())
            .unwrap(),
        http::Response::builder()
            .status(200)
            .header("ETag", "\"123\"")
            .body(SdkBody::from("hello"))
            .unwrap(),
    );
    let replay_client = StaticReplayClient::new(vec![req1]);
    let client = test_client(replay_client.clone());

    let file = File::<String>::loaded("test-bucket", "test-prefix", &client)
        .await
        .expect("Failed to fetch file");

    replay_client.assert_requests_match(&[]);
    assert_eq!(file.as_content().map(|f| f.as_str()), Some("hello"));
}

#[tokio::test]
async fn test_refetching_file_no_modified() {
    let req1 = ReplayEvent::new(
        http::Request::builder()
            .method("GET")
            .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
            .body(SdkBody::empty())
            .unwrap(),
        http::Response::builder()
            .status(200)
            .header("ETag", "\"123\"")
            .body(SdkBody::from("hello"))
            .unwrap(),
    );

    let req2 = ReplayEvent::new(
        http::Request::builder()
            .method("GET")
            .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
            .header("If-None-Match", "\"123\"")
            .body(SdkBody::empty())
            .unwrap(),
        http::Response::builder()
            .status(304)
            .body(SdkBody::empty())
            .unwrap(),
    );
    let replay_client = StaticReplayClient::new(vec![req1, req2]);
    let client = test_client(replay_client.clone());

    let file = File::<String>::loaded("test-bucket", "test-prefix", &client)
        .await
        .expect("Failed to fetch file");

    assert_eq!(file.as_content().map(|f| f.as_str()), Some("hello"));

    let file = file
        .fetch_data(&client)
        .await
        .expect("Failed to fetch file");

    assert_eq!(None, file);

    replay_client.assert_requests_match(&[]);
}

#[tokio::test]
async fn test_refetching_file_after_being_modified() {
    let req1 = ReplayEvent::new(
        http::Request::builder()
            .method("GET")
            .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
            .body(SdkBody::empty())
            .unwrap(),
        http::Response::builder()
            .status(200)
            .header("ETag", "\"123\"")
            .body(SdkBody::from("hello"))
            .unwrap(),
    );

    let req2 = ReplayEvent::new(
        http::Request::builder()
            .method("GET")
            .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
            .header("If-None-Match", "\"123\"")
            .body(SdkBody::empty())
            .unwrap(),
        http::Response::builder()
            .status(304)
            .body(SdkBody::empty())
            .unwrap(),
    );

    let req3 = ReplayEvent::new(
        http::Request::builder()
            .method("GET")
            .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
            .header("If-None-Match", "\"123\"")
            .body(SdkBody::empty())
            .unwrap(),
        http::Response::builder()
            .status(200)
            .header("ETag", "\"125\"")
            .body(SdkBody::from("bye"))
            .unwrap(),
    );

    let replay_client = StaticReplayClient::new(vec![req1, req2, req3]);
    let client = test_client(replay_client.clone());

    let file = File::<String>::loaded("test-bucket", "test-prefix", &client)
        .await
        .expect("Failed to fetch file");

    assert_eq!(file.as_content().map(|f| f.as_str()), Some("hello"));

    let not_modified = file
        .fetch_data(&client)
        .await
        .expect("Failed to fetch file");
    assert_eq!(None, not_modified);

    let modified = file
        .fetch_data(&client)
        .await
        .expect("Failed to fetch file");
    assert_eq!(
        Some("bye"),
        modified
            .as_ref()
            .and_then(|f| f.as_content())
            .map(|f| f.as_str())
    );

    replay_client.assert_requests_match(&[]);
}

mod parsing {
    use std::ops::Deref;

    use super::*;

    #[tokio::test]
    async fn test_can_parse_as_vec_u8() {
        let req1 = ReplayEvent::new(
            http::Request::builder()
                .method("GET")
                .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .header("ETag", "\"123\"")
                .body(SdkBody::from("hello"))
                .unwrap(),
        );
        let replay_client = StaticReplayClient::new(vec![req1]);
        let client = test_client(replay_client.clone());

        let file = File::<Vec<u8>>::loaded("test-bucket", "test-prefix", &client)
            .await
            .expect("Failed to fetch file");

        replay_client.assert_requests_match(&[]);
        assert_eq!(
            file.as_content().map(|f| f.as_slice()),
            Some(b"hello".as_slice())
        );
    }

    #[tokio::test]
    async fn test_can_parse_as_bytes() {
        let req1 = ReplayEvent::new(
            http::Request::builder()
                .method("GET")
                .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .header("ETag", "\"123\"")
                .body(SdkBody::from("hello"))
                .unwrap(),
        );
        let replay_client = StaticReplayClient::new(vec![req1]);
        let client = test_client(replay_client.clone());

        let file = File::<bytes::Bytes>::loaded("test-bucket", "test-prefix", &client)
            .await
            .expect("Failed to fetch file");

        replay_client.assert_requests_match(&[]);
        assert_eq!(
            file.as_content().map(|f| f.deref()),
            Some(&bytes::Bytes::from("hello"))
        );
    }
}
