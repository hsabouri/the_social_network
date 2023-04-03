use anyhow::Error;
use config::ServerConfig;
use dashmap::DashMap;
use futures::{FutureExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use models::users::{UserRef, Userlike};
use scylla::Session;
use sqlx::PgPool;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status};

use proto::social_network_server::SocialNetwork;
use proto::{
    FriendRequest, FriendResponse, Message, MessageRequest, MessageStatusResponse,
    NotificationsRequest, NotificationsResponse, PostMessageRequest, TimelineRequest,
    TimelineResponse,
};

use crate::connections::ServerConnections;

#[derive(Clone)]
pub struct ServerState {
    notifications: Arc<DashMap<String, broadcast::Sender<Message>>>,
    connections: ServerConnections,
    config: ServerConfig,
}

impl ServerState {
    pub async fn new(config: ServerConfig) -> Result<Self, Error> {
        Ok(Self {
            notifications: Arc::new(DashMap::new()),
            connections: ServerConnections::new(&config).await?,
            config,
        })
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

    async fn tag_read_message(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<MessageStatusResponse>, Status> {
        todo!()
    }

    async fn tag_unread_message(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<MessageStatusResponse>, Status> {
        todo!()
    }

    async fn post_message(
        &self,
        request: Request<PostMessageRequest>,
    ) -> Result<Response<MessageStatusResponse>, Status> {
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
            message_id: "FIXME".to_string(),
            read: false,
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

        let response = MessageStatusResponse { success: true };

        Ok(Response::new(response))
    }

    type TimelineStream = Pin<Box<dyn Stream<Item = Result<TimelineResponse, Status>> + Send>>;

    async fn timeline(
        &self,
        request: Request<TimelineRequest>,
    ) -> Result<Response<Self::TimelineStream>, Status> {
        let timeline_request = request.into_inner();
        let user = UserRef::from_str_uuid(timeline_request.user_id)
            .map_err(|_| Status::invalid_argument("Malformed user Uuid"))?;

        let stream = user
            .get_timeline(self.connections.get_pg(), &self.connections.get_scylla())
            .await
            .map_err(|e| Status::internal(format!("{e}")))?
            .map_ok(|message| TimelineResponse {
                messages: vec![Message {
                    user_id: message.user_id.to_string(),
                    message_id: message.id.to_string(),
                    timestamp: message.date.timestamp() as u64,
                    content: message.content,
                    read: false,
                }],
            })
            .map_err(|e| Status::internal(format!("{e}")));

        Ok(Response::new(Box::pin(stream)))
    }

    type RealTimeNotificationsStream =
        Pin<Box<dyn Stream<Item = Result<NotificationsResponse, Status>> + Send>>;

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
        let stream = BroadcastStream::new(rx)
            .map_ok(|message| NotificationsResponse {
                message: Some(message),
            })
            .map_err(|e| Status::internal(format!("error: {e}")));

        println!("User {user_id} connected to live notifications.");

        Ok(Response::new(Box::pin(stream)))
    }
}
