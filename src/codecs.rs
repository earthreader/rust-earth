#![unstable]

use std::borrow::{IntoCow, ToOwned};
use std::default::Default;
use std::str::FromStr;
use std::string::CowString;

use chrono::{DateTime, FixedOffset};
use chrono::{Timelike, Offset};

use schema::{SchemaResult, Codec};
use schema::SchemaError::{EncodeError, DecodeError};

macro_rules! try_encode {
    ($e:expr) => (match $e { Ok(v) => v, Err(_e) => return Err(EncodeError) })
}

macro_rules! try_opt {
    ($e:expr, $msg:expr) => (
        match $e { Some(e) => e,
                   None => return Err(DecodeError($msg, None)) }
    );
    ($e:expr, $msg:expr, $detail:expr) => (
        match $e { Some(e) => e,
                   None => return Err(DecodeError($msg, Some($detail))) }
    )
}

macro_rules! parse_field {
    ($caps:expr, $field:expr) => (
        {
            let value = $caps.name($field);
            try_opt!(value.and_then(FromStr::from_str),
                     concat!("invalid value for ", $field),
                     format!("{:?}", value))
        }
    )
}

#[allow(missing_copy_implementations)]
pub struct RFC3339;

impl Codec<DateTime<FixedOffset>> for RFC3339 {
    fn encode(&self, value: &DateTime<FixedOffset>, w: &mut Writer) -> SchemaResult<()> {
        let dt = value.format("%Y-%m-%dT%H:%M:%S");
        try_encode!(write!(w, "{}", dt));
        let nsec = value.nanosecond();
        if nsec != 0 {
            let nsec = format!("{:06}", nsec);
            try_encode!(write!(w, ".{}", nsec.trim_right_matches('0')));
        }
        let off_d = value.offset().local_minus_utc();
        if off_d.is_zero() {
            try_encode!(write!(w, "Z"));
        } else {
            let min = off_d.num_minutes();
            let (h, m) = (min / 60, min % 60);
            try_encode!(write!(w, "{h:+03}:{m:02}", h=h, m=m));
        }
        Ok(())
    }

    fn decode(&self, r: &str) -> SchemaResult<DateTime<FixedOffset>> {
        let pattern = regex!(concat!(
            r#"^\s*"#,
            r#"(?P<year>\d{4})-(?P<month>0[1-9]|1[012])-(?P<day>0[1-9]|[12]\d|3[01])"#,
            r#"T"#,
            r#"(?P<hour>[01]\d|2[0-3]):(?P<minute>[0-5]\d)"#,
                                   r#":(?P<second>[0-5]\d|60)(?:\.(?P<microsecond>\d+))?"#,
            r#"(?P<tz>Z|(?P<tz_offset>(?P<tz_offset_sign>[+-])(?P<tz_offset_hour>[01]\d|2[0-3])"#,
                                                          r#":(?P<tz_offset_minute>[0-5]\d)))"#,
            r#"\s*$"#,
        ));
        let caps = match pattern.captures(r) {
            None => {
                return Err(DecodeError("invalid RFC 3339 date time string",
                                       Some(r.to_owned())));
            }
            Some(c) => c,
        };
        let offset = if caps.name("tz_offset").map_or(false, |x| x.len() > 0) {
            let tz_hour: i32 = caps.name("tz_offset_hour").and_then(FromStr::from_str).unwrap();
            let tz_minute = caps.name("tz_offset_minute").and_then(FromStr::from_str).unwrap();
            let tz_sign = if caps.name("tz_offset_sign").map_or(false, |x| x == "+") { 1 } else { -1 };
            FixedOffset::east(tz_sign * (tz_hour * 60 + tz_minute) * 60)
        } else {
            FixedOffset::east(0)  // UTC
        };
        let mut microsecond = caps.name("microsecond").unwrap_or("").to_string();
        for _ in range(0, 6 - microsecond.len()) {
            microsecond.push('0');
        }
        let dt = offset.ymd(
                parse_field!(caps, "year"),
                parse_field!(caps, "month"),
                parse_field!(caps, "day"))
            .and_hms_micro(
                parse_field!(caps, "hour"),
                parse_field!(caps, "minute"),
                parse_field!(caps, "second"),
                try_opt!(FromStr::from_str(&*microsecond),
                         "invalid value for microsecond",
                         format!("{:?}", microsecond)));
        Ok(dt)
    }
}

pub struct Boolean {
    true_texts: Vec<CowString<'static>>,
    false_texts: Vec<CowString<'static>>,
    default_value: bool
}

impl Boolean {
    pub fn new<T, U>(true_texts: &[T], false_texts: &[U],
                     default_value: bool) -> Boolean
        where T: IntoCow<'static, String, str> + Clone,
              U: IntoCow<'static, String, str> + Clone
    {
        Boolean {
            true_texts: true_texts.iter().map(|t| t.clone().into_cow())
                                  .collect(),
            false_texts: false_texts.iter().map(|t| t.clone().into_cow())
                                    .collect(),
            default_value: default_value
        }
    }
}

impl Default for Boolean {
    fn default() -> Boolean {
        Boolean {
            true_texts: vec!["true".into_cow()],
            false_texts: vec!["false".into_cow()],
            default_value: false
        }
    }
}

impl Codec<bool> for Boolean {
    fn encode(&self, value: &bool, w: &mut Writer) -> SchemaResult<()> {
        match write!(w, "{}", match *value {
            true => &self.true_texts[0],
            false => &self.false_texts[0]
        }) {
            Ok(()) => Ok(()),
            Err(_) => Err(EncodeError),  // TODO
        }
    }

    fn decode(&self, r: &str) -> SchemaResult<bool> {
        if self.true_texts.iter().any(|&: t| &t[] == r) {
            Ok(true)
        } else if self.false_texts.iter().any(|&: f| &f[] == r) {
            Ok(false)
        } else if r.is_empty() {
            Ok(self.default_value)
        } else {
            Err(DecodeError("invalid string", None))
        }
    }
}

#[cfg(test)]
mod test {
    use super::RFC3339;
    use std::io::MemWriter;
    use std::str;
    use chrono::{DateTime, FixedOffset};
    use chrono::{Offset};
    use schema::{Codec};

    fn sample_data() -> Vec<(&'static str, DateTime<FixedOffset>)> {
        vec![
            ("2005-07-31T12:29:29Z",
             FixedOffset::east(0).ymd(2005, 7, 31).and_hms(12, 29, 29)),
            ("2003-12-13T18:30:02.25Z",
             FixedOffset::east(0).ymd(2003, 12, 13).and_hms_micro(18, 30, 2, 250000)),
            ("2003-12-13T18:30:02+01:00",
             FixedOffset::east(1 * 60 * 60).ymd(2003, 12, 13).and_hms(18, 30, 2)),
            ("2003-12-13T18:30:02.25+01:00",
             FixedOffset::east(1 * 60 * 60).ymd(2003, 12, 13).and_hms_micro(18, 30, 2, 250000)),
        ]
    }

    /*
    @mark.parametrize(('rfc3339_string', 'dt'), sample_data)
        def test_rfc3339_decode(rfc3339_string, dt):
        parsed = Rfc3339().decode(rfc3339_string)
        assert parsed == dt
        assert parsed.tzinfo.utcoffset(parsed) == dt.tzinfo.utcoffset(parsed)
        utc_parsed = Rfc3339(prefer_utc=True).decode(rfc3339_string)
        assert utc_parsed == dt
        assert utc_parsed.tzinfo.utcoffset(parsed) == datetime.timedelta(0)
     */
    #[test]
    fn test_rfc3339_decode() {
        for &(rfc3339_str, ref tm) in sample_data().iter() {
            let parsed = RFC3339.decode(rfc3339_str).unwrap();
            assert_eq!(parsed, *tm);
        }
    }

    fn to_string<T, C: Codec<T>>(codec: C, value: T) -> String {
        let mut w = MemWriter::new();
        codec.encode(&value, &mut w).unwrap();
        str::from_utf8(w.get_ref()).unwrap().to_string()
    }

    #[test]
    fn test_rfc3339_encode() {
        for &(rfc3339_str, ref dt) in sample_data().iter() {
            assert_eq!(to_string(RFC3339, (*dt).clone()), rfc3339_str);
            // TODO: assert (Rfc3339(prefer_utc=True).encode(dt) == codec.encode(dt.astimezone(utc)))
        }
    }
/*

def test_rfc3339_with_white_spaces():
    codec = Rfc3339()

    rfc_string = '''
        2003-12-13T18:30:02+01:00
    '''
    rfc_datetime = datetime.datetime(2003, 12, 13, 18, 30, 2,
                                     tzinfo=FixedOffset(60))

    assert codec.decode(rfc_string) == rfc_datetime
*/
    #[test]
    fn test_rfc3339_with_white_spaces() {
        let rfc_str = r#"
            2003-12-13T18:30:02+01:00
        "#;
        let dt = FixedOffset::east(1 * 60 * 60).ymd(2003, 12, 13).and_hms(18, 30, 2);
        let decoded_dt = RFC3339.decode(rfc_str).unwrap();
        assert_eq!(decoded_dt, dt);
    }
}
