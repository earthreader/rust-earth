use std::default::Default;

use chrono::{DateTime, FixedOffset};

use super::{Category, LinkList, Person, Text};

pub struct Metadata {
    pub id: String,
    pub title: Text,
    pub links: LinkList,
    pub updated_at: DateTime<FixedOffset>,
    pub authors: Vec<Person>,
    pub contributors: Vec<Person>,
    pub categories: Vec<Category>,
    pub rights: Option<Text>,
}

impl Metadata {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Metadata {
        Metadata {
            id: id,
            title: title,
            updated_at: updated_at,
            ..Default::default()
        }            
    }
}

impl Default for Metadata {
    fn default() -> Metadata {
        use chrono::{DateTime, NaiveDateTime};
        let default_datetime = DateTime::from_utc(
            NaiveDateTime::from_num_seconds_from_unix_epoch(0, 0),
            FixedOffset::east(0)
        );
        Metadata {
            id: Default::default(),
            title: Default::default(),
            links: Default::default(),
            updated_at: default_datetime,
            authors: Default::default(),
            contributors: Default::default(),
            categories: Default::default(),
            rights: Default::default(),
        }
    }
}
