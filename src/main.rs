use rusoto_core::credential::{AwsCredentials, DefaultCredentialsProvider, StaticProvider};
use rusoto_core::Region;
use rusoto_s3::{ListObjectsRequest, S3Client, S3};

#[tokio::main]
async fn main() {
    let list_request = ListObjectsRequest {
        delimiter: Some("/".to_owned()),
        bucket: "classting-archive".to_owned(),
        ..Default::default()
    };

    let client = S3Client::new_with(
        rusoto_core::request::HttpClient::new().unwrap(),
        DefaultCredentialsProvider::new().unwrap(),
        Region::ApNortheast1,
    );

    let resp = client
        .list_objects(list_request)
        .await
        .expect("list objects failed");

    let objs = resp.contents.unwrap();
    objs.iter()
        .for_each(|m| println!("{}", m.key.as_ref().unwrap()));
    resp.common_prefixes
        .unwrap()
        .iter()
        .for_each(|m| println!("{}", m.prefix.as_ref().unwrap()));
}
