use eyre::Result;
use std::sync::Arc;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, Region};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use crate::{FrontendEvent, RuntimeState, S3ClientEvent, S3Item};

pub struct S3Client {
    client: Client,
    runtime_state: Arc<Mutex<RuntimeState>>,
    event_sender: Sender<S3ClientEvent>,
    event_receiver: Receiver<FrontendEvent>,
}

impl S3Client {
    pub async fn new(
        runtime_state: Arc<Mutex<RuntimeState>>,
        event_sender: Sender<S3ClientEvent>,
        event_receiver: Receiver<FrontendEvent>,
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
            event_receiver,
        })
    }

    async fn enter(&mut self, item: S3Item) -> Result<()> {
        let mut runtime_state = self.runtime_state.lock().await;

        if let Some((bucket_name, prefix)) = match item {
            S3Item::Bucket(b) => Some((b.name().unwrap_or_default().to_owned(), String::default())),
            S3Item::Directory(d) => Some((
                runtime_state.bucket().unwrap_or_default(),
                d.prefix().unwrap_or_default().to_owned(),
            )),
            S3Item::Pop => Some((
                runtime_state.bucket().unwrap_or_default(),
                runtime_state.pop_prefix(),
            )),
            _ => None,
        } {
            let list_output = self
                .client
                .list_objects()
                .bucket(&bucket_name)
                .delimiter("/")
                .prefix(&prefix)
                .send()
                .await?;

            runtime_state.set_bucket(bucket_name);
            runtime_state.set_prefix(&prefix);
            runtime_state.set_items(S3Item::from_list_output(&list_output).0);

            self.event_sender.send(S3ClientEvent::Completed).await?;
        }

        Ok(())
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.event_receiver.recv().await {
            match event {
                FrontendEvent::End => {
                    break;
                }
                FrontendEvent::Refesh => {
                    // TODO: Refresh
                }
                // TODO: log
                FrontendEvent::Enter(item) => self.enter(item).await.expect("enter error"),
            }
        }
    }
}
