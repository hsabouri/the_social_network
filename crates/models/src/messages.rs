use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Message {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date_bucket: NaiveDate,
    pub date: NaiveDateTime,
    pub content: String,
}
