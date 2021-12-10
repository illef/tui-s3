use eyre::Result;
use std::sync::Arc;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, Region};
use tokio::sync::{mpsc::Sender, Mutex};

use crate::RuntimeState;
use crate::S3ClientEvent;

pub struct S3Client {
    client: Client,
    runtime_state: Arc<Mutex<RuntimeState>>,
    event_sender: Sender<S3ClientEvent>,
}

impl S3Client {
    pub async fn new(
        runtime_state: Arc<Mutex<RuntimeState>>,
        event_sender: Sender<S3ClientEvent>,
    ) -> Result<S3Client> {
        let region_provider =
            RegionProviderChain::default_provider().or_else(Region::new("ap-northeast-1"));
        let config = aws_config::from_env().region(region_provider).load().await;

        let client = Client::new(&config);

        let buckets = client.list_buckets().send().await?;

        // bucket list 를 생성할 때 초기화 한다
        runtime_state.lock().await.set_items(
            buckets
                .buckets()
                .unwrap_or_default()
                .into_iter()
                .map(|b| b.clone().into())
                .collect(),
        );

        event_sender.send(S3ClientEvent::Completed).await?;

        Ok(S3Client {
            client,
            runtime_state,
            event_sender,
        })
    }
}
