use crate::users::UserId;

pub enum FriendUpdate {
    New(UserId),
    Removed(UserId),
}

pub enum FriendshipUpdate {
    New(UserId, UserId),
    Removed(UserId, UserId),
}