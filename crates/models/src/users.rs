use std::{fmt::Display, str::FromStr};

use anyhow::Error;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct UserId(Uuid);

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for UserId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

impl From<Uuid> for UserId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl Into<Uuid> for UserId {
    fn into(self) -> Uuid {
        self.0
    }
}

impl UserId {
    pub fn try_parse(s: impl AsRef<str>) -> Result<Self, Error> {
        s.as_ref().parse()
    }
}

pub trait Userlike: Sized {
    fn get_id(&self) -> UserId;
}

impl Userlike for UserId {
    fn get_id(&self) -> UserId {
        *self
    }
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: UserId,
    pub name: String,
}
