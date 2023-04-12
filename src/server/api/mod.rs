use anyhow::Error;
use futures::{Stream, StreamExt, TryFutureExt, TryStreamExt};
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use config::ServerConfig;
use models::messages::{Message, MessageRef, Messagelike};
use models::users::{User, UserRef, Userlike};
use proto::social_network_server::SocialNetwork;
use proto::*;
use task_manager::TaskManager;

use crate::connections::ServerConnections;

mod helpers;

use helpers::*;

#[derive(Clone)]
pub struct ServerState {
    connections: ServerConnections,
    task_manager: TaskManager,
    _config: ServerConfig,
}

impl ServerState {
    pub async fn new(config: ServerConfig) -> Result<Self, Error> {
        Ok(Self {
            connections: ServerConnections::new(&config).await?,
            task_manager: TaskManager::new(),
            _config: config,
        })
    }
}

#[tonic::async_trait]
impl SocialNetwork for ServerState {
    async fn get_user_by_name(
        &self,
        request: Request<UserByNameRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let request = request.into_inner();

        let user = User::get_by_name(request.name)
            .execute(self.connections.get_pg())
            .await
            .map_err(Status::error_internal)?;

        Ok(Response::new(UserResponse {
            name: user.name,
            user_id: user.id.to_string(),
        }))
    }

    async fn add_friend(
        &self,
        request: Request<FriendRequest>,
    ) -> Result<Response<FriendResponse>, Status> {
        let request = request.into_inner();

        let user =
            UserRef::from_str_uuid(request.user_id).map_err(Status::error_invalid_argument)?;
        let friend =
            UserRef::from_str_uuid(request.friend_id).map_err(Status::error_invalid_argument)?;

        let connections = self.connections.clone();

        self.task_manager
            .spawn_await_result(async move {
                let realtime = user
                    .realtime_friend_with(friend)
                    .publish(connections.get_nats());

                let persistence = user
                    .friend_with(friend)
                    .execute(connections.get_pg())
                    .map_err(|e| Status::internal(e.to_string()));

                let (_rt, persistance) = futures::join!(realtime, persistence);
                persistance
            })
            .await?;

        Ok(Response::new(FriendResponse { success: true }))
    }

    async fn remove_friend(
        &self,
        request: Request<FriendRequest>,
    ) -> Result<Response<FriendResponse>, Status> {
        let request = request.into_inner();

        let user =
            UserRef::from_str_uuid(request.user_id).map_err(Status::error_invalid_argument)?;
        let friend =
            UserRef::from_str_uuid(request.friend_id).map_err(Status::error_invalid_argument)?;

        let connections = self.connections.clone();

        self.task_manager
            .spawn_await_result(async move {
                user.remove_friend(friend)
                    .execute(connections.get_pg())
                    .map_err(|e| Status::internal(e.to_string()))
                    .await
            })
            .await?;

        Ok(Response::new(FriendResponse { success: true }))
    }

    async fn post_message(
        &self,
        request: Request<PostMessageRequest>,
    ) -> Result<Response<MessageStatusResponse>, Status> {
        let request = request.into_inner();
        let preview = request
            .content
            .chars()
            .take(15)
            .chain("[…]".chars())
            .collect::<String>();

        println!(
            r#"User {} posted a new message: "{}""#,
            request.user_id, preview
        );

        let user =
            UserRef::from_str_uuid(request.user_id).map_err(Status::error_invalid_argument)?;

        let message = Message::new(user, request.content);

        let connections = self.connections.clone();

        self.task_manager
            .spawn_await_result(async move {
                let realtime = message
                    .clone()
                    .realtime_publish()
                    .publish(connections.get_nats());

                let persistence = message
                    .insert()
                    .execute(connections.get_scylla())
                    .map_err(Status::error_internal);

                let (_rt, persistance) = futures::join!(realtime, persistence);
                persistance
            })
            .await?;

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
            .map_err(Status::error_invalid_argument)?;

        let connections = self.connections.clone();

        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            let mut stream = user
                .get_timeline(connections.get_pg(), connections.get_scylla())
                .await
                .map_ok(|message| TimelineResponse {
                    messages: vec![message.into()],
                })
                .map_err(Status::error_internal);

            while let Some(item) = stream.next().await {
                let _ = tx.send(item).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }

    async fn tag_read_message(
        &self,
        request: Request<MessageTagRequest>,
    ) -> Result<Response<MessageStatusResponse>, Status> {
        let request = request.into_inner();

        let user =
            UserRef::from_str_uuid(request.user_id).map_err(Status::error_invalid_argument)?;

        let message = MessageRef::from_str_uuid(request.message_id)
            .map_err(Status::error_invalid_argument)?;

        let connections = self.connections.clone();

        self.task_manager
            .spawn_await_result(async move {
                message
                    .seen_by(user)
                    .execute(connections.get_scylla())
                    .map_err(|e| Status::internal(e.to_string()))
                    .await
            })
            .await?;

        Ok(Response::new(MessageStatusResponse { success: true }))
    }

    async fn tag_unread_message(
        &self,
        request: Request<MessageTagRequest>,
    ) -> Result<Response<MessageStatusResponse>, Status> {
        let request = request.into_inner();

        let user =
            UserRef::from_str_uuid(request.user_id).map_err(Status::error_invalid_argument)?;

        let message = MessageRef::from_str_uuid(request.message_id)
            .map_err(Status::error_invalid_argument)?;

        let connections = self.connections.clone();

        self.task_manager
            .spawn_await_result(async move {
                message
                    .unseen_by(user)
                    .execute(connections.get_scylla())
                    .map_err(|e| Status::internal(e.to_string()))
                    .await
            })
            .await?;

        Ok(Response::new(MessageStatusResponse { success: true }))
    }

    type RealTimeNotificationsStream =
        Pin<Box<dyn Stream<Item = Result<NotificationsResponse, Status>> + Send>>;

    async fn real_time_notifications(
        &self,
        request: Request<NotificationsRequest>,
    ) -> Result<Response<Self::RealTimeNotificationsStream>, Status> {
        let request = request.into_inner();
        let user =
            UserRef::from_str_uuid(&request.user_id).map_err(Status::error_invalid_argument)?;

        let connections = self.connections.clone();

        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            let stream = user
                .real_time_timeline(connections.get_pg(), connections.get_nats())
                .map_err(Status::error_internal)
                .map_ok(|message| NotificationsResponse {
                    message: Some(message.into()),
                });

            // FIXME: Remove this Box::pin
            let mut stream = Box::pin(stream);

            while let Some(item) = stream.next().await {
                let _ = tx.send(item).await;
            }
            // Client disconnected
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }
}
