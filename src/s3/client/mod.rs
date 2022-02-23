use eyre::Result;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    model::{Bucket, BucketLocationConstraint},
    output::ListObjectsV2Output,
    Client, Region,
};
pub struct S3Client {
    client: Client,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BucketWithLocation {
    pub location: BucketLocationConstraint,
    pub bucket: Bucket,
}

impl S3Client {
    pub async fn list_buckets(&self) -> Result<Vec<BucketWithLocation>> {
        let output = self.client.list_buckets().send().await?;
        if let Some(buckets) = output.buckets() {
            // bucket 과 location을 함께 구한다
            let location_bucket_list = futures::future::join_all(
                buckets
                    .into_iter()
                    .map(|b| (b.name(), b.to_owned()))
                    .filter(|(name, _)| name.is_some())
                    .map(|(name, bucket)| {
                        let get_location = self
                            .client
                            .get_bucket_location()
                            .bucket(name.unwrap())
                            .send();
                        let bucket = futures_util::future::ready(bucket);
                        futures_util::future::join(get_location, bucket)
                    }),
            )
            .await;
            Ok(location_bucket_list
                .into_iter()
                .filter(|(location, _)| location.is_ok())
                .map(|(location, bucket)| {
                    (
                        location.unwrap().location_constraint().unwrap().to_owned(),
                        bucket,
                    )
                })
                .map(|(location, bucket)| BucketWithLocation { location, bucket })
                .collect())
        } else {
            Ok(vec![])
        }
    }

    pub async fn list_objects(&self, bucket: &str, prefix: &str) -> Result<ListObjectsV2Output> {
        let output = self
            .client
            .list_objects_v2()
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
