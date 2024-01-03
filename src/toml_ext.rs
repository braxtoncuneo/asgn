use std::{any::type_name, fs, path::Path, str::FromStr};

use chrono::{Datelike, Timelike};

use crate::{error::Error, asgn_spec::DEFAULT_DUE_TIME};

pub trait TomlDatetimeExt: Sized {
    // Takes self by value because `toml::value::Datetime` is `Copy`
    fn try_into_chrono_date_time(self) -> Option<chrono::DateTime<chrono::Local>>;
}

impl TomlDatetimeExt for toml::value::Datetime {
    /// Returns `None` if there is no date component.
    fn try_into_chrono_date_time(self) -> Option<chrono::DateTime<chrono::Local>> {
        let date = {
            let toml::value::Date { year, month, day } = self.date?;
            chrono::NaiveDate::from_ymd_opt(year as _, month as _, day as _).unwrap()
        };

        let time = match self.time {
            None => DEFAULT_DUE_TIME,
            Some(time) => chrono::naive::NaiveTime::from_hms_opt(
                time.hour.into(),
                time.minute.into(),
                time.second.into(),
            ).unwrap(),
        };

        Some(date.and_time(time).and_local_timezone(chrono::Local).unwrap())
    }
}

pub trait ChronoDateTimeExt: Sized {
    // Takes self by ref because chrono::DateTime is not Copy
    fn to_toml_datetime(&self) -> toml::value::Datetime;
}

impl ChronoDateTimeExt for chrono::DateTime<chrono::Local> {
    fn to_toml_datetime(&self) -> toml::value::Datetime {
        let date = {
            let chrono_date = self.date_naive();
            Some(toml::value::Date {
                year:  chrono_date.year()  as _,
                month: chrono_date.month() as _,
                day:   chrono_date.day()   as _,
            })
        };

        let time = {
            let chrono_time = self.time();
            Some(toml::value::Time {
               hour:   chrono_time.hour()   as _,
               minute: chrono_time.minute() as _,
               second: chrono_time.second() as _,
               nanosecond: 0,
           })
        };

        toml::value::Datetime { date, time, offset: None }
    }
}

pub fn parse_date_to_chrono(date: &str) -> Result<chrono::DateTime<chrono::Local>, Error> {
    toml::value::Datetime::from_str(date).ok()
        .and_then(|toml_date| toml_date.try_into_chrono_date_time())
        .ok_or_else(|| Error::invalid_date(date))
}

pub fn parse_file<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, Error> {
    let text = fs::read_to_string(&path).map_err(|err|
        Error::io("Failed to read TOML file", &path, err)
    )?;

    toml::from_str(&text).map_err(|err|
        Error::invalid_toml(path, err)
    )
}

pub fn write_file<T: serde::ser::Serialize>(value: &T, path: impl AsRef<Path>) -> Result<(), Error> {
    let toml_text = toml::to_string(value).map_err(|err|
        Error::toml_ser(type_name::<T>(), err)
    )?;

    fs::write(&path, toml_text).map_err(|err|
        Error::io("Failed to write TOML file", path, err)
    )
}
