use eyre::Result;

use tui_aws::{
    frontend::run_frontend,
    glue::{client::get_all_glue_tables, controller::Controller},
};

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: error handle
    let glue_tables = get_all_glue_tables().await?;
    let controller = Controller::new(glue_tables);
    let ui_task = tokio::task::spawn(async move {
        run_frontend(controller).await.expect("frontend error");
    });

    ui_task.await?;

    Ok(())
}
