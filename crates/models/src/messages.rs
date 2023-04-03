use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::repository::messages::{InsertMessageRequest, SeenMessageRequest};

#[derive(Clone, Debug)]
pub struct Message {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: NaiveDateTime,
    pub content: String,
}

impl Message {
    pub fn insert(user_id: Uuid, content: String) -> InsertMessageRequest {
        InsertMessageRequest::new(user_id, content)
    }

    pub fn seen_by(&self, user_id: Uuid) -> SeenMessageRequest {
        SeenMessageRequest::new(self.id, user_id)
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.date == other.date
    }
}

impl Eq for Message {}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.date.partial_cmp(&other.date)
    }
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
