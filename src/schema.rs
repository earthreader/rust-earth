use time;

pub type SchemaResult<T> = Result<T, SchemaError>;

pub enum SchemaError {
    DescriptorConflict,
    IntegrityError,
    EncodeError,
    DecodeError(String),
}

pub trait Codec<E> {
    fn encode(&self, w: &mut Writer) -> Result<(), E>;
    fn decode(r: &str) -> Result<Self, E>;
}

macro_rules! try_opt(
    ($e:expr, $msg:expr) => (match $e { Some(e) => e, None => return Err(DecodeError($msg)) })
)

macro_rules! parse_field(
    ($caps:expr, $field:expr) => (
        {
            let value = $caps.name($field);
            try_opt!(from_str(value),
                     format!(concat!("invalid value for ", $field, ": {}"), value))
        }
    )
)

impl Codec<SchemaError> for time::Tm {
    fn encode(&self, w: &mut Writer) -> SchemaResult<()> {
        let dt = self.rfc3339();
        match write!(w, "{}", dt) {
            Ok(_) => Ok(()),
            Err(_e) => Err(EncodeError),
        }
    }

    fn decode(r: &str) -> SchemaResult<time::Tm> {
        let pattern = regex!(concat!(
            r#"^"#,
            r#"(?P<year>\d{4})-(?P<month>0[1-9]|1[012])-(?P<day>0[1-9]|[12]\d|3[01])"#,
            r#"T"#,
            r#"(?P<hour>[01]\d|2[0-3]):(?P<minute>[0-5]\d)"#,
                                   r#":(?P<second>[0-5]\d|60)(?:\.(?P<microsecond>\d+))?"#,
            r#"(?P<tz>Z|(?P<tz_offset>(?P<tz_offset_sign>[+-])(?P<tz_offset_hour>[01]\d|2[0-3])"#,
                                                          r#":(?P<tz_offset_minute>[0-5]\d)))$"#,
        ));
        let caps = match pattern.captures(r) {
            None => { return Err(DecodeError(format!("\"{}\" is not valid RFC 3339 date time string", r))); }
            Some(c) => c,
        };
        let tzinfo = if caps.name("tz_offset").len() > 0 {
            let tz_hour: i32 = from_str(caps.name("tz_offset_hour")).unwrap();
            let tz_minute = from_str(caps.name("tz_offset_minute")).unwrap();
            let tz_sign = if caps.name("tz_offset_sign") == "+" { 1 } else { -1 };
            tz_sign * (tz_hour * 60 + tz_minute)
        } else {
            0  // UTC
        };
        let mut microsecond = caps.name("microsecond").to_string();
        for _ in range(0, 6 - microsecond.len()) {
            microsecond.push_char('0');
        }
        let tm = time::Tm {
            tm_year: parse_field!(caps, "year"),
            tm_mon: { let m: i32 = parse_field!(caps, "month"); m + 1 },
            tm_mday: parse_field!(caps, "day"),
            tm_yday: 0,  // TODO
            tm_wday: 0,  // TODO
            tm_hour: parse_field!(caps, "hour"),
            tm_min: parse_field!(caps, "minute"),
            tm_sec: parse_field!(caps, "second"),
            tm_isdst: 0,  // TODO
            tm_gmtoff: tzinfo * 60,
            tm_nsec: {
                let msec: i32 = try_opt!(from_str(microsecond.as_slice()),
                                         format!("invalid value for microsecond: {}", microsecond));
                msec * 1000
            },
        };
        Ok(tm)
    }
}

pub trait Mergeable {
    fn merge_entities(self, other: Self) -> Self;
}
