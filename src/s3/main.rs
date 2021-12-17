use eyre::Result;

use structopt::StructOpt;
use tui_aws::{
    run_frontend,
    s3::controller::{Controller, Opt},
};

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let ui_task = tokio::task::spawn(async move {
        let controller = Controller::new(opt).await.expect("TODO: error handle");
        run_frontend(controller).await.expect("frontend error");
    });

    ui_task.await?;
    Ok(())
}
