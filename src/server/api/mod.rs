use anyhow::Error;
use futures::{Stream, StreamExt, TryFutureExt, TryStreamExt};
use std::pin::Pin;
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

        let realtime = user
            .realtime_friend_with(friend)
            .publish(self.connections.get_nats());

        let persistence = user
            .friend_with(friend)
            .execute(self.connections.get_pg())
            .map_err(|e| Status::internal(e.to_string()));

        self.task_manager
            .spawn_await_result(async move {
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

        let persistence = user
            .remove_friend(friend)
            .execute(self.connections.get_pg())
            .map_err(|e| Status::internal(e.to_string()));

        self.task_manager.spawn_await_result(persistence).await?;

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

        let realtime = message
            .clone()
            .realtime_publish()
            .publish(self.connections.get_nats());

        let persistence = message
            .insert()
            .execute(self.connections.get_scylla())
            .map_err(Status::error_internal);

        self.task_manager
            .spawn_await_result(async move {
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

        let stream = user
            .get_timeline(self.connections.get_pg(), &self.connections.get_scylla())
            .await
            .map_ok(|message| TimelineResponse {
                messages: vec![proto::Message {
                    user_id: message.user_id.to_string(),
                    message_id: message.id.to_string(),
                    timestamp: message.date.timestamp() as u64,
                    content: message.content,
                    read: false,
                }],
            })
            .map_err(Status::error_internal);

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

        let persistence = message
            .seen_by(user)
            .execute(self.connections.get_scylla())
            .map_err(|e| Status::internal(e.to_string()));

        self.task_manager.spawn_await_result(persistence).await?;

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

        let persistence = message
            .unseen_by(user)
            .execute(self.connections.get_scylla())
            .map_err(|e| Status::internal(e.to_string()));

        self.task_manager.spawn_await_result(persistence).await?;

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

        // FIXME: Errors in the friends stream are discarded.
        let stream = user
            .real_time_timeline(self.connections.get_pg(), self.connections.get_nats())
            .map_err(Status::error_internal)
            .map_ok(|message| NotificationsResponse {
                message: Some(message.into()),
            })
            .map_err(Status::error_internal);

        Ok(Response::new(Box::pin(stream)))
    }
}
