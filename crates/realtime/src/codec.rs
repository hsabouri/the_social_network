use std::str::FromStr;

use thiserror::Error;
use prost::Message as ProstMessage;

use models::users::*;
use models::messages::*;

#[derive(Error, Debug)]
pub enum ProtoDecodingError {
    #[error("invalid protobuf payload")]
    Prost(#[from] prost::DecodeError),
    #[error("invalid Message")]
    Message(#[from] models::proto::ProtoDecodeMessageError),
    #[error("invalid UserId")]
    UserId(#[from] UserIdParsingError),
    #[error("invalid UserId")]
    MessageId(#[from] MessageIdParsingError),
}

pub(crate) fn decode_proto_message(payload: prost::bytes::Bytes) -> Result<Message, ProtoDecodingError> {
    let m = proto::Message::decode(payload)?;

    let message = Message::try_from(m)?;

    Ok(message)
}

pub(crate) fn decode_proto_friendship(
    payload: prost::bytes::Bytes,
) -> Result<(UserId, UserId), ProtoDecodingError> {
    let friendship = proto::Friendship::decode(payload)?;

    let user = UserId::from_str(friendship.user.as_str())?;
    let friend = UserId::from_str(friendship.friend.as_str())?;

    Ok((user, friend))
}

pub(crate) fn decode_proto_message_tag_request(
    payload: prost::bytes::Bytes,
) -> Result<(UserId, MessageId), ProtoDecodingError> {
    let tag = proto::MessageTagRequest::decode(payload)?;

    let user = UserId::from_str(tag.user_id.as_str())?;
    let message = MessageId::try_parse(tag.message_id.as_str())?;

    Ok((user, message))
}

pub(crate) fn encode_proto_message(message: Message) -> prost::bytes::Bytes {
    let m: proto::Message = message.into();

    m.encode_to_vec().into()
}

pub(crate) fn encode_proto_message_tag_request(
    user: UserId,
    message: MessageId,
) -> prost::bytes::Bytes {
    let m = proto::MessageTagRequest {
        user_id: user.get_id().to_string(),
        message_id: message.get_id().to_string(),
    };

    m.encode_to_vec().into()
}

pub(crate) fn encode_proto_friendship(user: UserId, friend: UserId) -> prost::bytes::Bytes {
    let m = proto::Friendship {
        user: user.get_id().to_string(),
        friend: friend.get_id().to_string(),
    };

    m.encode_to_vec().into()
}
