use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, Error, Region};

#[tokio::main]
async fn main() {
    // TODO: args 가 1 보다 작을 시 usage 메시지를 남길 것
    let bucket = std::env::args().collect::<Vec<String>>()[1].clone();

    // TODO: args 로 region을 선택받을 수 있게 할 것
    let region_provider =
        RegionProviderChain::default_provider().or_else(Region::new("ap-northeast-1"));
    let shared_config = aws_config::from_env().region(region_provider).load().await;

    let client = Client::new(&shared_config);

    let resp = client
        .list_objects_v2()
        .bucket(&bucket)
        .delimiter("/")
        .send()
        .await
        .unwrap();

    println!("Objects:");

    for object in resp.common_prefixes().unwrap_or_default() {
        println!("{}", object.prefix().unwrap_or_default());
    }

    for object in resp.contents().unwrap_or_default() {
        println!("{}", object.key().unwrap_or_default());
    }
}
