use eyre::Result;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    output::{ListBucketsOutput, ListObjectsOutput},
    Client, Region,
};
pub struct S3Client {
    client: Client,
}

impl S3Client {
    pub async fn list_buckets(&self) -> Result<ListBucketsOutput> {
        let output = self.client.list_buckets().send().await?;
        Ok(output)
    }

    pub async fn list_objects(&self, bucket: &str, prefix: &str) -> Result<ListObjectsOutput> {
        let output = self
            .client
            .list_objects()
            .bucket(bucket)
            .delimiter("/")
            .prefix(prefix)
            .send()
            .await?;

        Ok(output)
    }

    pub async fn new() -> Result<S3Client> {
        let region_provider =
            RegionProviderChain::default_provider().or_else(Region::new("ap-northeast-1"));
        let config = aws_config::from_env().region(region_provider).load().await;

        let client = Client::new(&config);

        Ok(S3Client { client })
    }
}
