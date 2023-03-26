use anyhow::Error;
use futures::stream::StreamExt;
use futures::{FutureExt, Stream, TryStreamExt};
use tonic::transport::Channel;

use tsn::social_network_client::SocialNetworkClient;
use tsn::{
    FriendRequest, Message, NotificationsRequest, PostMessageRequest, TimelineRequest,
};

/// Placeholder authentication system. It is used to store the user_id along with the gRPC client.
#[derive(Clone, Debug)]
pub struct Connector<T = SocialNetworkClient<Channel>> {
    user_id: String,
    _inner: T,
}

impl Connector<SocialNetworkClient<Channel>> {
    pub async fn handle_notifs(self) -> Result<(), Error> {
        let request = NotificationsRequest {
            user_id: self.user_id.clone(),
        };

        let mut stream = self
            ._inner
            .clone()
            .real_time_notifications(request)
            .await?
            .into_inner();

        println!("✅ Subscribed to real-time notifications");

        while let Some(notification) = stream.next().await {
            notification?.message.map(|message| {
                println!(
                    "{} a posté un nouveau message : {}",
                    message.user_id, message.content
                );
            });
        }

        println!("Closed notification stream.");

        Ok(())
    }

    pub async fn add_friend(self, friend_id: String) -> Result<(), Error> {
        let request = FriendRequest {
            user_id: self.user_id.clone(),
            friend_id,
        };

        let response = self._inner.clone().add_friend(request).await?.into_inner();

        match response.success {
            true => Ok(()),
            false => Err(Error::msg("Server returned an error").context("calling `add_friend`")),
        }
    }

    pub async fn rm_friend(self, friend_id: String) -> Result<(), Error> {
        let request = FriendRequest {
            user_id: self.user_id.clone(),
            friend_id,
        };

        let response = self
            ._inner
            .clone()
            .remove_friend(request)
            .await?
            .into_inner();

        match response.success {
            true => Ok(()),
            false => Err(Error::msg("Server returned an error").context("calling `remove_friend`")),
        }
    }

    pub async fn post_message(self, content: String) -> Result<(), Error> {
        let request = PostMessageRequest {
            user_id: self.user_id.clone(),
            content,
        };

        let response = self
            ._inner
            .clone()
            .post_message(request)
            .await?
            .into_inner();

        match response.success {
            true => Ok(()),
            false => Err(Error::msg("Server returned an error").context("calling `post_`")),
        }
    }

    pub async fn get_timeline_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Vec<Message>, Error>>, Error> {
        let request = TimelineRequest {
            user_id: self.user_id.clone(),
        };

        let stream = self._inner.clone().timeline(request).await?.into_inner();

        let stream = stream.map(|response| match response {
            Ok(response) => Ok(response.messages),
            Err(e) => Err(Error::msg(format!("error {e}"))),
        });

        Ok(stream)
    }
}

pub trait Auth<T> {
    fn auth(self, user_id: String) -> Result<Connector<T>, Error>;
}

impl<T> Auth<SocialNetworkClient<T>> for SocialNetworkClient<T> {
    fn auth(self, user_id: String) -> Result<Connector<Self>, Error> {
        Ok(Connector {
            user_id,
            _inner: self,
        })
    }
}
