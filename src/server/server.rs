use anyhow::Error;
use dashmap::DashMap;
use futures::{FutureExt, Stream, TryFutureExt, TryStreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{transport::Server, Request, Response, Status};

use tsn::social_network_server::{SocialNetwork, SocialNetworkServer};
use tsn::{
    FriendRequest, FriendResponse, Message, MessageResponse, NotificationsRequest,
    NotificationsResponse, PostMessageRequest, TimelineRequest, TimelineResponse,
};

#[derive(Clone)]
struct ServerState {
    notifications: Arc<DashMap<String, broadcast::Sender<Message>>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(DashMap::new()),
        }
    }

    async fn broadcast_message(&self, message: Message) -> Result<(), Error> {
        let user_id = &message.user_id;
        // Do not send message to OP
        let subscribed_users = self
            .notifications
            .iter()
            .filter(|user| user.key() != user_id);

        for sub_user in subscribed_users {
            // Push and forget
            println!("Broadcasting message to {}", sub_user.key());
            let _ = sub_user.value().send(message.clone());
        }

        Ok(())
    }
}

#[tonic::async_trait]
impl SocialNetwork for ServerState {
    async fn add_friend(
        &self,
        request: Request<FriendRequest>,
    ) -> Result<Response<FriendResponse>, Status> {
        todo!()
    }

    async fn remove_friend(
        &self,
        request: Request<FriendRequest>,
    ) -> Result<Response<FriendResponse>, Status> {
        todo!()
    }

    async fn post_message(
        &self,
        request: Request<PostMessageRequest>,
    ) -> Result<Response<MessageResponse>, Status> {
        let message = request.into_inner();
        let preview = message
            .content
            .chars()
            .take(15)
            .chain("[...]".chars())
            .collect::<String>();

        println!(
            r#"User {} posted a new message: "{}""#,
            message.user_id, preview
        );

        let message = Message {
            user_id: message.user_id,
            content: message.content,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        // TODO: Store in DB

        // Stream to other connected users.
        let f = self.broadcast_message(message).await;
        match f {
            Err(e) => println!("Error while broadcasing message: {e}"),
            _ => (),
        };

        let response = MessageResponse { success: true };

        Ok(Response::new(response))
    }

    type TimelineStream =
        Pin<Box<dyn Stream<Item = Result<TimelineResponse, Status>> + Send + Sync>>;

    async fn timeline(
        &self,
        _request: Request<TimelineRequest>,
    ) -> Result<Response<Self::TimelineStream>, Status> {
        // TODO: This is a placeholder
        let messages = vec![
            Message {
                user_id: "user1".to_string(),
                content: "Ceci est un message d'exemple 1.".to_string(),
                timestamp: 1,
            },
            Message {
                user_id: "user2".to_string(),
                content: "Ceci est un message d'exemple 2.".to_string(),
                timestamp: 2,
            },
            Message {
                user_id: "user3".to_string(),
                content: "Ceci est un message d'exemple 3.".to_string(),
                timestamp: 3,
            },
        ];

        let stream = futures::stream::iter(messages.into_iter().map(|message| {
            let response = TimelineResponse {
                messages: vec![message],
            };
            Ok(response)
        }));

        Ok(Response::new(Box::pin(stream)))
    }

    type RealTimeNotificationsStream = Pin<Box<dyn Stream<Item = Result<NotificationsResponse, Status>> + Send>>;

    async fn real_time_notifications(
        &self,
        request: Request<NotificationsRequest>,
    ) -> Result<Response<Self::RealTimeNotificationsStream>, Status> {
        let request = request.into_inner();
        let user_id = request.user_id;

        self.notifications
            .entry(user_id.clone())
            .or_insert_with(|| {
                // Receiver will be created from sender.
                // Multiple receiver can exist for on Sender (user is connected on multiple sessions)
                let (tx, _) = broadcast::channel(100);
                tx
            });

        let rx = self.notifications.get(&user_id).unwrap().subscribe();
        let stream = BroadcastStream::new(rx).map_ok(|message| {
            NotificationsResponse {
                message: Some(message),
            }
        }).map_err(|e| Status::internal(format!("error: {e}")));

        println!("User {user_id} connected to live notifications.");

        Ok(Response::new(Box::pin(stream)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let server_state = ServerState::new();

    Server::builder()
        .add_service(SocialNetworkServer::new(server_state))
        .serve(addr)
        .await?;

    Ok(())
}
