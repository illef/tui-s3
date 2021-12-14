use eyre::Result;

use tui_s3::s3::{frontend::run_frontend, Opt};

#[tokio::main]
async fn main() -> Result<()> {
    // let opt = Opt::from_args();

    let ui_task = tokio::task::spawn(async move {
        run_frontend().await.expect("frontend error");
    });

    ui_task.await?;
    Ok(())
}
