use aws_sdk_s3::model::{CommonPrefix, Object};
use eyre::Result;

use std::sync::Arc;
use tokio::sync::Mutex;

use tui_s3::frontend::run_frontend;
use tui_s3::RuntimeState;

#[tokio::main]
async fn main() -> Result<()> {
    let runtime_state = Arc::new(Mutex::new(RuntimeState::new()));

    {
        runtime_state.lock().await.set_common_prefixes(Some(
            vec!["folder1/", "folder2/"]
                .into_iter()
                .map(|f| {
                    CommonPrefix::builder()
                        .set_prefix(Some(f.to_owned()))
                        .build()
                })
                .collect(),
        ));

        runtime_state.lock().await.set_contents(Some(
            vec!["key1", "key2"]
                .into_iter()
                .map(|f| Object::builder().set_key(Some(f.to_owned())).build())
                .collect(),
        ))
    }

    let ui_task = tokio::task::spawn(async move {
        run_frontend(runtime_state.clone())
            .await
            .expect("frontend error");
    });

    ui_task.await?;
    Ok(())
}
