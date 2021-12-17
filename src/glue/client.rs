use std::sync::Arc;

use aws_sdk_glue::model::{Database, Table};
use eyre::Result;

#[derive(Debug)]
pub struct GlueTable {
    pub database: Arc<Database>,
    pub table: Table,
}

pub async fn get_all_glue_tables() -> Result<Vec<GlueTable>> {
    let shared_config = aws_config::load_from_env().await;
    let client = aws_sdk_glue::Client::new(&shared_config);

    if let Some(database_list) = client.get_databases().send().await?.database_list() {
        let database_table_list = futures::future::join_all(database_list.into_iter().map(|d| {
            let tables = client
                .get_tables()
                .set_database_name(d.name().map(|n| n.to_owned()))
                .send();
            let database = futures_util::future::ready(d.to_owned());
            futures_util::future::join(database, tables)
        }))
        .await;

        Ok(database_table_list
            .into_iter()
            .filter_map(|(database, table_output)| table_output.ok().map(|t| (database, t)))
            .filter_map(|(database, table_output)| {
                table_output
                    .table_list()
                    .map(|table_list| (database, table_list.to_owned()))
            })
            .map(|(database, table_list)| {
                let database = Arc::new(database);
                table_list.into_iter().map(move |t| GlueTable {
                    database: database.clone(),
                    table: t,
                })
            })
            .flatten()
            .collect())
    } else {
        Err(eyre::eyre!("No database in Glue"))
    }
}
