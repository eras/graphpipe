mod graph;
mod layout;
mod server;
mod stable_ids;

use env_logger::Env;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("error"));
    crate::server::main().await
}
