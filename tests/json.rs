#[cfg(any(feature = "simd-json", feature = "json"))]
mod parsing {
    use std::ops::Deref;

    use aws_sdk_s3::{
        config::{Credentials, Region},
        primitives::SdkBody,
        Client, Config,
    };
    use aws_smithy_runtime::client::http::test_util::{ReplayEvent, StaticReplayClient};

    use conditional_s3_fetch::File;
    use conditional_s3_fetch::Json;

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

    #[derive(serde::Deserialize, Eq, PartialEq, Debug)]
    struct MyStruct {
        key: String,
    }

    #[tokio::test]
    async fn test_parsing_json() {
        let req1 = ReplayEvent::new(
            http::Request::builder()
                .method("GET")
                .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .header("ETag", "\"123\"")
                .body(SdkBody::from(r#"{"key": "value"}"#))
                .unwrap(),
        );
        let replay_client = StaticReplayClient::new(vec![req1]);
        let client = test_client(replay_client.clone());

        let file = File::<Json<MyStruct>>::loaded("test-bucket", "test-prefix", &client)
            .await
            .expect("Failed to fetch file");

        replay_client.assert_requests_match(&[]);
        assert_eq!(
            file.as_content().map(|f| f.deref()),
            Some(&MyStruct {
                key: "value".to_string()
            })
        );
    }

    #[tokio::test]
    async fn test_parsing_failure() {
        let req1 = ReplayEvent::new(
            http::Request::builder()
                .method("GET")
                .uri("https://test-bucket.s3.us-east-1.amazonaws.com/test-prefix?x-id=GetObject")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .header("ETag", "\"123\"")
                .body(SdkBody::from(r#"bad data"#))
                .unwrap(),
        );
        let replay_client = StaticReplayClient::new(vec![req1]);
        let client = test_client(replay_client.clone());

        let file = File::<Json<MyStruct>>::loaded("test-bucket", "test-prefix", &client).await;

        replay_client.assert_requests_match(&[]);

        assert!(matches!(
            file,
            Err(conditional_s3_fetch::Error::ParseError(_))
        ));
    }
}
