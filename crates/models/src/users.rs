use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}
