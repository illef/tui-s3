use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, Region};
use std::io::Write;

use futures_util::stream::StreamExt;
use tokio::io;
use tokio_util::codec::{FramedRead, LinesCodec};

use tui_s3::RuntimeState;

async fn get_line() -> Result<String, Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut reader = FramedRead::new(stdin, LinesCodec::new());
    let line = reader.next().await.transpose()?.unwrap();
    Ok(line)
}

#[tokio::main]
async fn main() {
    // TODO: args 가 1 보다 작을 시 usage 메시지를 남길 것
    let bucket = std::env::args().collect::<Vec<String>>()[1].clone();

    // TODO: args 로 region을 선택받을 수 있게 할 것
    let region_provider =
        RegionProviderChain::default_provider().or_else(Region::new("ap-northeast-1"));
    let shared_config = aws_config::from_env().region(region_provider).load().await;

    let client = Client::new(&shared_config);

    let mut runtime_state = RuntimeState::new();

    loop {
        let resp = client
            .list_objects_v2()
            .bucket(&bucket)
            .delimiter("/")
            .prefix(runtime_state.prefix())
            .send()
            .await
            .unwrap();

        let common_prefix = resp
            .common_prefixes()
            .unwrap_or_default()
            .iter()
            .collect::<Vec<_>>();

        // print directory
        for (index, object) in common_prefix.iter().enumerate() {
            println!("{}: {}", index + 1, object.prefix().unwrap_or_default());
        }

        // print file
        for object in resp.contents().unwrap_or_default() {
            println!("{}", object.key().unwrap_or_default());
        }

        loop {
            print!(":");
            std::io::stdout().flush().unwrap();
            if let Ok(index) = get_line().await.unwrap().parse::<usize>() {
                if index == 0 {
                    runtime_state.pop();
                    println!("prefix : {}", runtime_state.prefix());
                } else if index - 1 < common_prefix.len() {
                    runtime_state.set_prefix(common_prefix[index - 1].prefix().unwrap_or_default());
                    println!("prefix : {}", runtime_state.prefix());
                }
                break;
            }
        }
    }
}
