//! Realtime features for users, all in form of streams.

mod channels;
mod codec;
pub mod receivers;
pub mod senders;

pub use receivers::*;
pub use senders::*;

#[cfg(test)]
#[tokio::test]
async fn message_broadcast_test() -> Result<(), Box<dyn std::error::Error>> {
    use async_nats::ConnectOptions;
    use futures::StreamExt;

    println!("Testing a simple NATS case: send/recv");

    let sender = tokio::spawn(async move {
        let sender = ConnectOptions::new()
            .connect("nats://ruser:password@127.0.0.1:4222")
            .await
            .unwrap();
        println!("SENDER: Connected to NATS: {}", sender.connection_state());

        for _ in 0..10 {
            let _ = tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let _ = sender.publish("notification".into(), "ouai".into()).await;
            println!("SENDER: Sent message");
        }
    });

    let receiver_1 = tokio::spawn(async move {
        let receiver = ConnectOptions::new()
            .connect("nats://ruser:password@127.0.0.1:4222")
            .await
            .unwrap();
        println!(
            "RECEIVER_1: Connected to NATS: {}",
            receiver.connection_state()
        );

        let mut sub = receiver.subscribe("notification".into()).await.unwrap();

        println!("RECEIVER_1: message: \"{:#?}\"", sub.next().await);
    });

    let receiver_2 = tokio::spawn(async move {
        let receiver = ConnectOptions::new()
            .connect("nats://ruser:password@127.0.0.1:4222")
            .await
            .unwrap();
        println!(
            "RECEIVER_2: Connected to NATS: {}",
            receiver.connection_state()
        );

        let mut sub = receiver.subscribe("notification".into()).await.unwrap();

        println!("RECEIVER_2: message: \"{:#?}\"", sub.next().await);
    });

    let _ = tokio::join!(receiver_1, receiver_2, sender);

    Ok(())
}
