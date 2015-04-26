use std::borrow::Cow;
use std::default::Default;
use std::io;

use chrono::{DateTime, FixedOffset};

use codecs;    
use parser::base::{DecodeResult, XmlElement};
use schema::{Codec, Entity, FromSchemaReader, Mergeable};

/// Represent whether the entry is read, starred, or tagged by user.
///
/// It's not a part of [RFC 4287 Atom standard][rfc-atom], but extension
/// for Earth Reader.
///
/// [rfc-atom]: https://tools.ietf.org/html/rfc4287
#[derive(Clone, Default, PartialEq, Eq, Hash, Debug)]
pub struct Mark {
    /// Whether it's marked or not.
    pub marked: bool,

    /// Updated time.
    pub updated_at: Option<DateTime<FixedOffset>>,
}

impl Entity for Mark {
    type Id = ();

    /// If there are two or more marks that have the same tag name, these
    /// are all should be merged into one.
    fn entity_id(&self) -> Cow<()> { Cow::Owned(()) }
}

impl Mergeable for Mark {
    fn merge_with(&mut self, other: Mark) {
        use std::cmp::Ordering::Less;
        let cmp = self.updated_at.cmp(&other.updated_at);
        match cmp {
            Less => { self.clone_from(&other); }
            _    => { }
        }
    }
}

impl FromSchemaReader for Mark {
    fn read_from<B: io::BufRead>(&mut self, element: XmlElement<B>)
                                 -> DecodeResult<()>
    {
        self.updated_at = {
            let updated_at = try!(element.get_attr("updated"));
            Some(try!(codecs::RFC3339.decode(updated_at)))
        };
        let content = try!(element.read_whole_text());
        let codec: codecs::Boolean = Default::default();
        self.marked = try!(codec.decode(&content));
        Ok(())
    }        
}


#[cfg(test)]
mod test {
    use super::Mark;

    use chrono::{Offset, FixedOffset};

    fn fx_mark_true() -> Mark {
        Mark {
            marked: true,
            updated_at: Some(FixedOffset::east(0).ymd(2013, 11, 6)
                             .and_hms(14, 36, 0)),
        }
    }

    fn fx_mark_false() -> Mark {
        Mark {
            marked: false,
            updated_at: Some(FixedOffset::east(0).ymd(2013, 11, 6)
                             .and_hms(14, 36, 0)),
        }
    }
}
