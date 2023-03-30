use super::GTimeOffset;
use crate::write_locale;
use crate::error::{CURRENT_LOCALE, Locale};

use std::str::FromStr;

use chrono::{
    DateTime, Datelike, Timelike, Weekday, NaiveDate, Duration,
    offset::{Local, TimeZone},
    format,
    ParseError
};
use once_cell::sync::OnceCell;

static TIMESTAMP_20000101_IN_SECOND: OnceCell<i64> = OnceCell::new();
static TIMESTAMP_20000101_IN_MILLI: OnceCell<i64> = OnceCell::new();

struct GetTime {
    dt: DateTime<Local>,
}

impl GetTime {
    fn now() -> Self {
        let dt = Local::now().into();
        Self { dt }
    }
    fn from_str(dt: &str) -> GetTimeResult<Self> {
        let dt = match dt.len() {
            8 => {
                let s = format!("{dt}000000");
                Local.datetime_from_str(&s, "%Y%m%d%H%M%S")
            },
            10 => {
                let mut s = dt.replace("/", "-");
                s.push_str("000000");
                Local.datetime_from_str(&s, "%F%H%M%S")
            },
            14 => Local.datetime_from_str(dt, "%Y%m%d%H%M%S"),
            19 => {
                let s = dt.replace("/", "-");
                Local.datetime_from_str(&s, "%Y-%m-%d %T")
            },
            _ => DateTime::from_str(dt),
        }?;
        let gt = Self { dt };
        Ok(gt)
    }
    fn get_20000101_sec() -> i64 {
        let sec = TIMESTAMP_20000101_IN_SECOND.get_or_init(|| {
            let naive = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
            let local = Local.from_local_datetime(&naive).unwrap();
            local.timestamp()
        });
        *sec
    }
    fn get_20000101_milli() -> i64 {
        let milli = TIMESTAMP_20000101_IN_MILLI.get_or_init(|| {
            let naive = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
            let local = Local.from_local_datetime(&naive).unwrap();
            local.timestamp_millis()
        });
        *milli
    }
    fn from_seconds(secs: i64) -> GetTimeResult<Self> {
        let actual = secs + Self::get_20000101_sec();
        let dt = Local.timestamp_opt(actual, 0).single().ok_or(GetTimeError::InvalidSecond(secs))?;
        let gt = Self { dt };
        Ok(gt)
    }
    fn from_milliseconds(millis: i64) -> GetTimeResult<Self> {
        let actual = millis + Self::get_20000101_milli();
        let dt = Local.timestamp_millis_opt(actual).single().ok_or(GetTimeError::InvalidSecond(millis))?;
        let gt = Self { dt };
        Ok(gt)
    }
    fn millis(&self) -> i64 {
        self.dt.timestamp_millis() - Self::get_20000101_milli()
    }
    fn seconds(&self) -> i64 {
        self.dt.timestamp() - Self::get_20000101_sec()
    }
    fn to_duration(offset: f64, opt: GTimeOffset) -> Duration {
        let milliseconds = match opt {
            GTimeOffset::G_OFFSET_DAYS => offset * (24 * 60 * 60 * 1000) as f64,
            GTimeOffset::G_OFFSET_HOURS => offset * (60 * 60 * 1000) as f64,
            GTimeOffset::G_OFFSET_MINUTES => offset * (60 * 1000) as f64,
            GTimeOffset::G_OFFSET_SECONDS => offset * 1000 as f64,
            GTimeOffset::G_OFFSET_MILLIS => offset,
        } as i64;
        Duration::milliseconds(milliseconds)
    }
    fn set_duration(&mut self, duration: Duration) {
        self.dt += duration;
    }
    fn format(&self, fmt: &str) -> String {
        let locale = match *CURRENT_LOCALE {
            Locale::Jp => format::Locale::ja_JP,
            Locale::En => format::Locale::en_US,
        };
        self.dt.format_localized(fmt, locale).to_string()
    }
}

pub type GetTimeResult<T> = Result<T, GetTimeError>;
pub enum GetTimeError {
    ParseError(ParseError),
    InvalidSecond(i64),
    InvalidMilliSecond(i64)
}
impl From<ParseError> for GetTimeError {
    fn from(e: ParseError) -> Self {
        Self::ParseError(e)
    }
}

impl std::fmt::Display for GetTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetTimeError::ParseError(e) => write!(f, "{e}"),
            GetTimeError::InvalidSecond(s) => write_locale!(f,
                "{s}は有効な秒数ではありません",
                "{s} is not a valid second",
            ),
            GetTimeError::InvalidMilliSecond(m) => write_locale!(f,
                "{m}は有効なミリ秒数ではありません",
                "{m} is not a valid millisecond",
            ),
        }
    }
}

pub struct GetTimeValue {
    pub timestamp_millis: i64,
    pub timestamp_seconds: i64,
    pub year: i32,
    pub month: i32,
    pub date: i32,
    pub hour: i32,
    pub minute: i32,
    pub second: i32,
    pub millisec: i32,
    pub day: i32,
}
impl From<GetTime> for GetTimeValue {
    fn from(gt: GetTime) -> Self {
        let day = match gt.dt.weekday() {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        };
        Self {
            timestamp_millis: gt.millis(),
            timestamp_seconds: gt.seconds(),
            year: gt.dt.year(),
            month: gt.dt.month() as i32,
            date: gt.dt.day() as i32,
            hour: gt.dt.hour() as i32,
            minute: gt.dt.minute() as i32,
            second: gt.dt.second() as i32,
            millisec: gt.dt.timestamp_subsec_millis() as i32,
            day,
        }

    }
}

pub fn get(dt: Option<String>, offset: f64, opt: GTimeOffset) -> GetTimeResult<GetTimeValue> {
    let mut gt = match dt {
        Some(dt) => GetTime::from_str(&dt)?,
        None => GetTime::now(),
    };
    if offset != 0.0 {
        let duration = GetTime::to_duration(offset, opt);
        gt.set_duration(duration);
    }
    Ok(gt.into())
}

pub fn format(fmt: &str, secs: i64, milli: bool) -> GetTimeResult<String> {
    let gt = if milli {
        GetTime::from_milliseconds(secs)
    } else {
        GetTime::from_seconds(secs)
    }?;

    Ok(gt.format(fmt))
}