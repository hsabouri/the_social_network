use anyhow::Error;
use chrono::NaiveDateTime;
use uuid::Uuid;

use prost::Message as ProstMessage;

use crate::{
    messages::{Message, MessageRef},
    users::UserRef,
};

pub(crate) fn parse_proto_message(payload: prost::bytes::Bytes) -> Result<Message, Error> {
    let m = proto::Message::decode_length_delimited(payload)?;

    let message = Message {
        id: Uuid::try_parse(m.message_id.as_str())?,
        user_id: Uuid::try_parse(m.user_id.as_str())?,
        date: NaiveDateTime::from_timestamp_millis(m.timestamp as i64).unwrap(),
        content: m.content,
    };

    Ok(message)
}

pub(crate) fn parse_proto_friendship(
    payload: prost::bytes::Bytes,
) -> Result<(UserRef, UserRef), Error> {
    let friendship = proto::Friendship::decode_length_delimited(payload)?;

    let user = UserRef(Uuid::try_parse(friendship.user.as_str())?);
    let friend = UserRef(Uuid::try_parse(friendship.friend.as_str())?);

    Ok((user, friend))
}

pub(crate) fn parse_proto_message_tag_request(
    payload: prost::bytes::Bytes,
) -> Result<(UserRef, MessageRef), Error> {
    let tag = proto::MessageTagRequest::decode_length_delimited(payload)?;

    let user = UserRef(Uuid::try_parse(tag.user_id.as_str())?);
    let message = MessageRef(Uuid::try_parse(tag.message_id.as_str())?);

    Ok((user, message))
}
