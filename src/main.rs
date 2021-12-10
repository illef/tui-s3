use eyre::Result;

use tui_s3::frontend::run_frontend;

#[tokio::main]
async fn main() -> Result<()> {
    let ui_task = tokio::task::spawn_blocking(move || run_frontend());

    ui_task.await??;
    Ok(())
}
