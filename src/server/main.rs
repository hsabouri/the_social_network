use proto::social_network_server::SocialNetworkServer;
use tonic::transport::Server;
use clap::Parser;

mod api;
mod connections;

use api::ServerState;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("./config/config.dev.json"))]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = config::ServerConfig::load_from_file(args.config)?;

    let server_state = ServerState::new(config.clone()).await?;

    Server::builder()
        .add_service(SocialNetworkServer::new(server_state))
        .serve(config.listening_addr)
        .await?;

    Ok(())
}
