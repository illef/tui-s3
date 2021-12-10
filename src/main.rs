use eyre::Result;

use std::sync::Arc;
use tokio::sync::{mpsc::channel, Mutex};

use tui_s3::{frontend::run_frontend, s3::S3Client, RuntimeState};

#[tokio::main]
async fn main() -> Result<()> {
    let runtime_state = Arc::new(Mutex::new(RuntimeState::new()));

    let (s3_client_sender, s3_client_receiver) = channel(10);
    let (frontend_event_sender, frontend_event_receiver) = channel(10);

    let runtime_state_copy = runtime_state.clone();

    let ui_task = tokio::task::spawn(async move {
        run_frontend(
            runtime_state_copy,
            s3_client_receiver,
            frontend_event_sender,
        )
        .await
        .expect("frontend error");
    });

    let mut s3_client = S3Client::new(
        runtime_state.clone(),
        s3_client_sender,
        frontend_event_receiver,
    )
    .await?;

    tokio::spawn(async move {
        s3_client.run().await;
    });

    ui_task.await?;
    Ok(())
}
