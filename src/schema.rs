use time;

pub type SchemaResult<T> = Result<T, SchemaError>;

#[deriving(Show)]
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
            tm_year: { let y: i32 = parse_field!(caps, "year"); y - 1900 },
            tm_mon: { let m: i32 = parse_field!(caps, "month"); m - 1 },
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


#[cfg(test)]
mod test {
    use super::SchemaError;
    use super::Codec;
    use std::io::MemWriter;
    use std::str;
    use time;

    static sample_data: &'static [(&'static str, time::Tm)] = &[
        ("2005-07-31T12:29:29Z",
         time::Tm { tm_year: 2005 - 1900, tm_mon: 6, tm_mday: 31,
                    tm_yday: 0, tm_wday: 0,  // dummy
                    tm_hour: 12, tm_min: 29, tm_sec: 29, tm_nsec: 0,
                    tm_isdst: 0, tm_gmtoff: 0 }),
        ("2003-12-13T18:30:02.25Z",
         time::Tm { tm_year: 2003 - 1900, tm_mon: 11, tm_mday: 13,
                    tm_yday: 0, tm_wday: 0,  // dummy
                    tm_hour: 18, tm_min: 30, tm_sec: 2, tm_nsec: 250000 * 1000,
                    tm_isdst: 0, tm_gmtoff: 0 }),
        ("2003-12-13T18:30:02+01:00",
         time::Tm { tm_year: 2003 - 1900, tm_mon: 11, tm_mday: 13,
                    tm_yday: 0, tm_wday: 0,  // dummy
                    tm_hour: 18, tm_min: 30, tm_sec: 2, tm_nsec: 0,
                    tm_isdst: 0, tm_gmtoff: 1 * 60 * 60 }),
        ("2003-12-13T18:30:02.25+01:00",
         time::Tm { tm_year: 2003 - 1900, tm_mon: 11, tm_mday: 13,
                    tm_yday: 0, tm_wday: 0,  // dummy
                    tm_hour: 18, tm_min: 30, tm_sec: 2, tm_nsec: 250000 * 1000,
                    tm_isdst: 0, tm_gmtoff: 1 * 60 * 60 }),
    ];

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
        for &(rfc3339_str, tm) in sample_data.iter() {
            let parsed: time::Tm = Codec::<SchemaError>::decode(rfc3339_str).unwrap();
            assert_eq!(parsed, tm);
        }
    }
    
    fn to_string<T: Codec<E>, E>(value: T) -> String {
        let mut w = MemWriter::new();
        value.encode(&mut w);
        str::from_utf8(w.unwrap().as_slice()).unwrap().into_string()
    }

    #[test]
    fn test_rfc3339_encode() {
        for &(rfc3339_str, tm) in sample_data.iter() {
            assert_eq!(to_string(tm).as_slice(), rfc3339_str);
            // TODO: assert (Rfc3339(prefer_utc=True).encode(dt) == codec.encode(dt.astimezone(utc)))
        }
    }

    #[test]
    fn test_rfc3339_now() {
        let now = time::now();
        let encoded = to_string(now);
        let decoded: time::Tm = Codec::<SchemaError>::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded, now);
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
        let rfc_tm = time::Tm {
            tm_year: 2003 - 1900, tm_mon: 12 - 1, tm_mday: 13,
            tm_yday: 0, tm_wday: 0,  // dummy
            tm_hour: 18, tm_min: 30, tm_sec: 2, tm_nsec: 0,
            tm_isdst: 0, tm_gmtoff: 60 * 60
        };
        let decoded_tm: time::Tm = Codec::<SchemaError>::decode(rfc_str).unwrap();
        assert_eq!(decoded_tm, rfc_tm);
    }
}
