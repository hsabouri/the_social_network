use super::Connector;
use anyhow::Error;
use futures::{FutureExt, StreamExt};
use promptly::prompt;
use std::str::FromStr;

enum Action {
    Timeline,
    Post,
    AddFriend(String),
    RmFriend(String),
    Close,
}

impl FromStr for Action {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split_whitespace().collect();

        let action = *splitted
            .get(0)
            .ok_or_else(|| Error::msg(format!("Invalid action {s}")))?;
        let argument = splitted.get(1).map(|s| *s);

        match (action, argument) {
            ("timeline", _) => Ok(Self::Timeline),
            ("post", _) => Ok(Self::Post),
            ("add_friend", Some(s)) => Ok(Self::AddFriend(s.to_string())),
            ("rm_friend", Some(s)) => Ok(Self::RmFriend(s.to_string())),
            ("add_friend", None) => Err(Error::msg("Missing argument for action `add_friend`")),
            ("rm_friend", None) => Err(Error::msg("Missing argument for action `rm_friend`")),
            ("close", _) => Ok(Self::Close),
            (s, _) => Err(Error::msg(format!("Invalid action {s}"))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cli {
    client: Connector,
}

impl Cli {
    async fn post(&self) -> Result<(), Error> {
        let content: String = prompt("Write your post (enter to commit):")?;

        self.client.clone().post_message(content)
            .then(|res| async {
                match res.as_ref() {
                    Ok(_) => println!("✅ Successfully posted message"),
                    Err(e) => println!("❌ Error: {e}"),
                };
                res
            }).await
    }

    async fn timeline(&self) -> Result<(), Error> {
        let mut timeline_stream = self.client.clone().get_timeline_stream().await?;

        loop {
            println!("Gathering next posts...");

            let posts = timeline_stream.next().await;

            match posts {
                Some(Ok(posts)) => {
                    for post in posts {
                        println!("Post from {}: ({})", post.user_id, post.timestamp);
                        println!("{}", post.content);
                    }
                },
                Some(Err(e)) => {
                    println!("❌ Error: {e}");
                    break
                }
                None => {
                    println!("😭 No posts to see 😭 Try again later 😭");
                    break
                }
            }

            let action: bool = prompt("Continue timeline ?")?;

            if !action {
                break
            }
        }
        Ok(())
    }

    async fn add_friend(&self, friend_id: String) -> Result<(), Error> {
        println!("Sent `add_friend` request...");
        self.client
            .clone()
            .add_friend(friend_id.clone())
            .then(|res| async {
                match res.as_ref() {
                    Ok(_) => println!("✅ Successfully added friend {friend_id}"),
                    Err(e) => println!("❌ Error: {e}"),
                };
                res
            })
            .await
    }

    async fn rm_friend(&self, friend_id: String) -> Result<(), Error> {
        println!("Sent `remove_friend` request...");
        self.client
            .clone()
            .rm_friend(friend_id.clone())
            .then(|res| async {
                match res.as_ref() {
                    Ok(_) => println!("✅ Successfully removed friend {friend_id}"),
                    Err(e) => println!("❌ Error: {e}"),
                };
                res
            })
            .await
    }

    pub async fn interactivity_loop_inner(&self) -> Result<(), Error> {
        loop {
            let action: Result<Action, Error> = prompt::<String, &str>(
                "What do you want to do ? (timeline/post/add_friend/rm_friend/close)",
            )?
            .parse();
            let action = match action {
                Ok(action) => action,
                Err(e) => {
                    println!("Error: {e}");
                    continue;
                }
            };

            let res = match action {
                Action::Close => break,
                Action::AddFriend(id) => self.add_friend(id).await,
                Action::RmFriend(id) => self.rm_friend(id).await,
                Action::Post => self.post().await,
                Action::Timeline => self.timeline().await,
            };

            match res {
                Ok(_) => (),
                Err(e) => {
                    println!("Raised error: {e}");
                }
            }
        }

        Ok(())
    }

    pub async fn interactivity_loop(client: Connector) -> Result<(), Error> {
        let cli = Self { client: client };

        cli.interactivity_loop_inner().await
    }
}
