use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

use crate::repository::messages::InsertMessageRequest;

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
}
