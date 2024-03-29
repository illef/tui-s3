use eyre::Result;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    model::{Bucket, BucketLocationConstraint},
    output::ListObjectsV2Output,
    Client, Endpoint, Region,
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
        let mut next_continuation_token: Option<String> = None;
        let mut object_list = vec![];
        let mut common_prefixes = vec![];

        loop {
            let list_output = self
                .client
                .list_objects_v2()
                .set_continuation_token(next_continuation_token.clone())
                .bucket(bucket)
                .delimiter("/")
                .prefix(prefix)
                .send()
                .await?;

            let (contents, prefixes, token) = (
                list_output.contents,
                list_output.common_prefixes,
                list_output.next_continuation_token,
            );

            next_continuation_token = token;
            if let Some(objects_vec) = contents {
                object_list.extend(objects_vec);
            }
            if let Some(prefixes) = prefixes {
                common_prefixes.extend(prefixes);
            }
            if next_continuation_token.is_none() {
                break;
            }
        }

        Ok(ListObjectsV2Output::builder()
            .set_contents(Some(object_list))
            .name(bucket)
            .prefix(prefix)
            .set_common_prefixes(Some(common_prefixes))
            .build())
    }

    pub async fn new(
        profile_name: Option<&String>,
        endpoint_url: Option<&String>,
    ) -> Result<S3Client> {
        use aws_config::profile::ProfileFileCredentialsProvider;
        let provider = ProfileFileCredentialsProvider::builder()
            .profile_name(
                profile_name
                    .map(|p| p.as_str())
                    .or(Some("default"))
                    .unwrap(),
            )
            .build();
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::from_env()
            .credentials_provider(provider)
            .region(region_provider);

        let config = if let Some(endpoint_url) = endpoint_url {
            let endpoint = Endpoint::immutable(endpoint_url.parse().expect("valid URI"));
            config.endpoint_resolver(endpoint)
        } else {
            config
        };

        let client = Client::new(&config.load().await);

        Ok(S3Client { client })
    }
}
