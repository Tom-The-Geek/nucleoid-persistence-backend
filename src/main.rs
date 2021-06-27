use xtra::Actor;
use xtra::spawn::Tokio;

mod database;
mod config;
mod web;
mod model;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let config = config::load();
    let database = database::MongoDatabaseHandler::connect(&config).await?
        .create(None)
        .spawn(&mut Tokio::Global);

    web::run(&config, database.clone()).await;

    Ok(())
}
