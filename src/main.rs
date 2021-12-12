use eyre::Result;

use tui_s3::frontend::run_frontend;

#[tokio::main]
async fn main() -> Result<()> {
    let ui_task = tokio::task::spawn(async move {
        run_frontend().await.expect("frontend error");
    });

    ui_task.await?;
    Ok(())
}
