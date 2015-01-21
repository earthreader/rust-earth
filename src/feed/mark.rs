#![unstable]

use chrono::{DateTime, FixedOffset};
    
use schema::{Mergeable};

/// Represent whether the entry is read, starred, or tagged by user.
///
/// It's not a part of [RFC 4287 Atom standard][rfc-atom], but extension
/// for Earth Reader.
///
/// [rfc-atom]: https://tools.ietf.org/html/rfc4287
#[derive(Default, PartialEq, Eq, Hash, Show)]
pub struct Mark {
    /// Whether it's marked or not.
    pub marked: bool,

    /// Updated time.
    pub updated_at: Option<DateTime<FixedOffset>>,
}

impl Mark {
    /// If there are two or more marks that have the same tag name, these
    /// are all should be merged into one.
    #[experimental = "should be exposed as a trait"]
    fn __entity_id__(&self) -> &str { "" }
}

impl Mergeable for Mark {
    fn merge_entities(self, other: Mark) -> Mark {
        use std::cmp::Ordering::Less;
        match self.updated_at.cmp(&other.updated_at) {
            Less => other,
            _    => self,
        }
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
