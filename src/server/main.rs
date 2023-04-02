use proto::social_network_server::SocialNetworkServer;
use tonic::transport::Server;

mod api;

use api::ServerState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::ServerConfig::load_from_file("./config/config.dev.json")?;

    let server_state = ServerState::new(config.clone());

    Server::builder()
        .add_service(SocialNetworkServer::new(server_state))
        .serve(config.listening_addr)
        .await?;

    Ok(())
}
