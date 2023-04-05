use anyhow::Error;
use futures::future;

use clap::Parser;

mod cli;
mod connector;

use cli::Cli;
use connector::*;
use proto::social_network_client::SocialNetworkClient;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("http://[::1]:50051"))]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    let name: String = asking::text()
        .message("What is you user name (Alice, Bob, Charlie) ?\n")
        .ask_and_wait()?;
    let client = SocialNetworkClient::connect(args.addr)
        .await?
        .auth_by_name(name)
        .await?;

    println!("Your UUID: {}", client.user_id);

    // Subscribe to real-time messages :
    let notifs = client.clone().handle_notifs();

    let f = Cli::interactivity_loop(client);

    match future::select(Box::pin(notifs), Box::pin(f)).await {
        future::Either::Left(_) => {
            // Handle disconections better. (try to reconnect)
            println!("❌ Disconnected from server.");
            Err(Error::msg("disconnected"))
        }
        future::Either::Right(_) => {
            println!("Bye !");
            Ok(())
        }
    }
}
