use anyhow::Error;
use uuid::Uuid;

use prost::Message as ProstMessage;

use crate::{
    messages::{Message, MessageRef, Messagelike},
    users::{UserRef, Userlike},
};

pub(crate) fn decode_proto_message(payload: prost::bytes::Bytes) -> Result<Message, Error> {
    let m = proto::Message::decode(payload)?;

    let message = Message::try_from(m)?;

    Ok(message)
}

pub(crate) fn decode_proto_friendship(
    payload: prost::bytes::Bytes,
) -> Result<(UserRef, UserRef), Error> {
    let friendship = proto::Friendship::decode(payload)?;

    let user = UserRef(Uuid::try_parse(friendship.user.as_str())?);
    let friend = UserRef(Uuid::try_parse(friendship.friend.as_str())?);

    Ok((user, friend))
}

pub(crate) fn decode_proto_message_tag_request(
    payload: prost::bytes::Bytes,
) -> Result<(UserRef, MessageRef), Error> {
    let tag = proto::MessageTagRequest::decode(payload)?;

    let user = UserRef(Uuid::try_parse(tag.user_id.as_str())?);
    let message = MessageRef(Uuid::try_parse(tag.message_id.as_str())?);

    Ok((user, message))
}

pub(crate) fn encode_proto_message(message: Message) -> prost::bytes::Bytes {
    let m: proto::Message = message.into();

    m.encode_to_vec().into()
}

pub(crate) fn encode_proto_message_tag_request(
    user: UserRef,
    message: MessageRef,
) -> prost::bytes::Bytes {
    let m = proto::MessageTagRequest {
        user_id: user.get_uuid().to_string(),
        message_id: message.get_uuid().to_string(),
    };

    m.encode_to_vec().into()
}
