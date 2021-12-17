use eyre::Result;

use tui_aws::glue::client::get_all_glue_tables;

#[tokio::main]
async fn main() -> Result<()> {
    println!("{:?}", get_all_glue_tables().await);

    Ok(())
}
