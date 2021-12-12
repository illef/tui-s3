use eyre::Result;
use std::sync::Arc;

use crate::{
    s3::S3Client,
    view_model::{S3ItemsViewModel, S3Output},
    S3Item,
};
use tokio::sync::{mpsc::Sender, Mutex};

pub struct S3ItemsViewModelController {
    // 컨트롤  대상
    vm: Arc<Mutex<S3ItemsViewModel>>,
    client: Arc<Mutex<S3Client>>,
    // UI를 다시 그릴것을 요청한다
    ev_tx: Sender<()>,
}

impl S3ItemsViewModelController {
    pub async fn new(ev_tx: Sender<()>) -> Result<Self> {
        Ok(Self {
            vm: Arc::new(Mutex::new(S3ItemsViewModel::new())),
            // TODO: 에러 처리
            client: Arc::new(Mutex::new(S3Client::new().await?)),
            ev_tx,
        })
    }

    pub fn view_model(&self) -> Arc<Mutex<S3ItemsViewModel>> {
        self.vm.clone()
    }

    pub async fn previous(&mut self) {
        self.vm.lock().await.previous();
    }

    pub async fn next(&mut self) {
        self.vm.lock().await.next();
    }

    pub async fn refresh(&mut self) {
        let bucket_and_prefix = { self.vm.lock().await.bucket_and_prefix() };
        let ev_tx_copy = self.ev_tx.clone();
        let vm_copy = self.vm.clone();
        let client_copy = self.client.clone();
        match bucket_and_prefix {
            None => {
                tokio::spawn(async move {
                    if let Ok(output) = client_copy.lock().await.list_buckets().await {
                        vm_copy.lock().await.update(S3Output::Buckets(output));
                        ev_tx_copy.send(()).await.expect("ev_tx_copy send error");
                    } else {
                        // TODO: error 처리
                    }
                });
            }
            Some((bucket, prefix)) => {
                tokio::spawn(async move {
                    if let Ok(output) = client_copy
                        .lock()
                        .await
                        .list_objects(&bucket, &prefix)
                        .await
                    {
                        vm_copy.lock().await.update(S3Output::Objects(output));
                        ev_tx_copy.send(()).await.expect("ev_tx_copy send error");
                    } else {
                        // TODO: error 처리
                    }
                });
            }
        }
    }

    pub async fn enter(&mut self) {
        let item = self.vm.lock().await.selected().map(|i| i.to_owned());
        let ev_tx_copy = self.ev_tx.clone();
        let vm_copy = self.vm.clone();
        let client_copy = self.client.clone();

        let bucket_and_prefix = match item {
            Some(S3Item::Pop) => {
                self.vm.lock().await.pop();
                None
            }
            Some(S3Item::Bucket(bucket_with_location)) => Some((
                bucket_with_location
                    .bucket
                    .name()
                    .map(|b| b.to_owned())
                    .unwrap(),
                "".to_owned(),
            )),
            Some(S3Item::Directory(d)) => Some((
                self.vm
                    .lock()
                    .await
                    .bucket_and_prefix()
                    .map(|b| b.0)
                    .unwrap(),
                d.prefix().map(|d| d.to_owned()).unwrap(),
            )),
            _ => None,
        };

        if let Some((bucket, prefix)) = bucket_and_prefix {
            tokio::spawn(async move {
                if let Ok(output) = client_copy
                    .lock()
                    .await
                    .list_objects(&bucket, &prefix)
                    .await
                {
                    vm_copy.lock().await.push(S3Output::Objects(output));
                    ev_tx_copy.send(()).await.expect("ev_tx_copy send error");
                } else {
                    // TODO: error 처리
                }
            });
        }
    }
}
