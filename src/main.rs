use eyre::Result;

use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio::sync::Mutex;

use tui_s3::frontend::run_frontend;
use tui_s3::s3::S3Client;
use tui_s3::RuntimeState;

#[tokio::main]
async fn main() -> Result<()> {
    let runtime_state = Arc::new(Mutex::new(RuntimeState::new()));

    let (s3_client_sender, s3_client_receiver) = channel(10);

    let runtime_state_copy = runtime_state.clone();

    let ui_task = tokio::task::spawn(async move {
        run_frontend(runtime_state_copy, s3_client_receiver)
            .await
            .expect("frontend error");
    });

    let s3_client = S3Client::new(runtime_state.clone(), s3_client_sender).await?;

    ui_task.await?;
    Ok(())
}
