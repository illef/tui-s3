use eyre::Result;

use structopt::StructOpt;
use tui_aws::s3::{
    controller::{Controller, Opt},
    frontend::run_frontend,
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
