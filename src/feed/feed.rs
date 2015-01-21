use std::borrow::ToOwned;
use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};
use xml;

use parser::base::NestedEventReader;
use parser::base::NestedEvent::Nested;
use schema::FromSchemaReader;

use super::{Entry, Source, Text};


#[derive(Default)]
pub struct Feed {
    pub source: Source,

    pub entries: Vec<Entry>,
}

impl Deref for Feed {
    type Target = Source;
    fn deref(&self) -> &Source { &self.source }
}

impl DerefMut for Feed {
    fn deref_mut(&mut self) -> &mut Source { &mut self.source }
}

impl Feed {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Feed {
        Feed {
            source: Source::new_inherited(id, title, updated_at),
            entries: Default::default(),
        }
    }

    pub fn new<T, S: ?Sized>(id: T, title: Text,
                             updated_at: DateTime<FixedOffset>) -> Feed
        where T: Deref<Target=S>, S: ToOwned<String>
    {
        Feed::new_inherited(id.to_owned(), title, updated_at)
    }
}

pub fn read_feed<B: Buffer>(buf: B) -> Feed {
    let mut parser = xml::EventReader::new(buf);
    let mut events = NestedEventReader::new(&mut parser);
    let mut feed: Feed = Default::default();
    for_each!(event in events.next() {
        match event {
            Nested { name: _, element } =>
                FromSchemaReader::read_from(&mut feed, element).unwrap(),
            _ => { }
        }
    });
    feed
}
