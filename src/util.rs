use itertools::Itertools;

use std::{
    fs::{self, Permissions},
    io::{Write, self},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    fmt::{self, Write as _},
    os::unix::fs::PermissionsExt,
    any::type_name,
};

use crate::error::Error;

use termion::terminal_size;
use walkdir::WalkDir;
use chrono::{Datelike, Timelike};

pub mod color {
    use termion::{color::*, style};

    pub const FG_RED: Fg<Red> = Fg(Red);
    pub const FG_GREEN: Fg<Green> = Fg(Green);
    pub const FG_YELLOW: Fg<Yellow> = Fg(Yellow);

    pub const BG_LIGHT_BLACK: Bg<LightBlack> = Bg(LightBlack);

    pub const TEXT_BOLD: style::Bold = style::Bold;
    pub const COLOR_REVERSED: style::Invert = style::Invert;
    pub const STYLE_RESET: style::Reset = style::Reset;
}

use self::color::*;

pub enum Hline { Normal, Bold }

impl fmt::Display for Hline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match terminal_size() {
            Ok((width, _)) => match self {
                Self::Normal => write!(f, "{:-<1$}", "", width as usize),
                Self::Bold => write!(f, "{TEXT_BOLD}{:=<1$}{STYLE_RESET}", "", width as usize),
            }
            Err(_) => Ok(()),
        }
    }
}

pub fn run_at(mut cmd: Command, path: impl AsRef<Path>, pipe_stdout: bool) -> Result<ExitStatus, Error> {
    let cmd = cmd.current_dir(path.as_ref());
    let program = cmd.get_program().to_owned().into_string().unwrap();
    let make_err = |err: io::Error| Error::command(&program, err);

    let output = if pipe_stdout {
        cmd
            .stdout(Stdio::inherit())
            .spawn()
            .map_err(make_err)?
            .wait_with_output()
            .map_err(make_err)?
    } else {
        cmd
            .output()
            .map_err(make_err)?
    };

    let status = output.status;

    Ok(status)
}

pub fn set_mode(path: impl AsRef<Path>, mode: u32) -> Result<(), Error> {
    fs::set_permissions(&path, Permissions::from_mode(mode)).map_err(|err|
        Error::io("Failed to chmod", &path, err)
    )
}

#[derive(Clone)]
pub struct FaclEntry {
    pub username: String,
    pub read: bool,
    pub write: bool,
    pub exe: bool,
}

impl fmt::Display for FaclEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:", self.username)?;
        if self.read  { f.write_char('r')?; }
        if self.write { f.write_char('w')?; }
        if self.exe   { f.write_char('x')?; }

        Ok(())
    }
}

pub fn set_facl<'facl>(
    path: impl AsRef<Path>,
    default: bool,
    mut entries: impl Iterator<Item=&'facl FaclEntry>,
) -> Result<(), Error>
{
    let entry_string = entries.join(",");

    if entry_string.is_empty() {
        return Ok(());
    }

    let mut cmd = std::process::Command::new("setfacl");

    if default {
        cmd.arg("-d");
    }

    cmd.arg("-m");
    cmd.arg(entry_string.clone()).arg(path.as_ref());

    let output = cmd.output().map_err(|err|
        Error::command("setfacl", err)
    )?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(Error::subprocess("setfacl", err_msg))
    }

    Ok(())
}



pub fn refresh_file(path: impl AsRef<Path>, mode: u32, default_text: &str) -> Result<(), Error> {
    let path = path.as_ref();

    if !path.exists() {
        fs::write(path, default_text).map_err(|err|
            Error::io("Failed to crete default file", path, err)
        )?;
    }

    set_mode(path, mode)?;

    Ok(())
}

pub fn refresh_dir<'facl>(
    path: impl AsRef<Path>,
    mode: u32,
    facl: impl Iterator<Item=&'facl FaclEntry> + Clone,
) -> Result<(), Error>
{
    let path = path.as_ref();

    if !path.exists() {
        fs::create_dir(path).map_err(|err|
            Error::io("Failed to create directory", path, err)
        )?;
    }

    set_mode(path, mode)?;

    set_facl(path, false, facl.clone())?;
    set_facl(path, true, facl)?;

    Ok(())
}

pub fn recursive_refresh_dir<'facl> (
    path: impl AsRef<Path>,
    mode: u32,
    facl: impl Iterator<Item=&'facl FaclEntry> + Clone,
) -> Result<(), Error>
{
    let path = path.as_ref();

    if !path.exists() {
        refresh_dir(path, mode, facl.clone())?;
        return Ok(());
    }

    if path.is_file() {
        return refresh_file(path, mode, "");
    } else if path.is_dir() {
        refresh_dir(path, mode, facl.clone())?;
    } else {
        return Ok(());
    }

    for maybe_entry in WalkDir::new(path).min_depth(1) {
        let dir_entry = maybe_entry.map_err(|err|
            Error::io("Failed to get directory entry", path, err.into())
        )?;
        recursive_refresh_dir(dir_entry.path(), mode, facl.clone())?;
    }

    Ok(())
}

pub fn bashrc_append_line(line: &str) -> Result<(), Error> {
    let home_path: PathBuf = dirs::home_dir().ok_or_else(Error::no_home_dir)?;

    let bashrc_path : PathBuf = home_path.join(".bashrc");
    let mut bashrc = fs::OpenOptions::new()
        .append(true)
        .open(&bashrc_path)
        .map_err(|err| Error::io("Failed to open bashrc", &bashrc_path, err))?;

    writeln!(bashrc, "{line}").map_err(|err|
        Error::io("Failed writing to bashrc", bashrc_path, err)
    )?;

    Ok(())
}

pub fn make_fresh_dir(path: &Path, base_name: &str) -> PathBuf {
    let mut idx: Option<usize> = None;

    let gen_path = |idx: Option<usize>| {
        if let Some(idx) = idx {
            path.join(format!("{}.{}", base_name, idx))
        } else {
            path.join(base_name)
        }
    };

    while gen_path(idx).exists() {
        idx = idx.map(|i| i + 1).or(Some(0));
    }

    gen_path(idx)
}

pub const DEFAULT_DUE_TIME: chrono::NaiveTime = chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap();

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

pub fn parse_toml_file<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, Error> {
    let text = fs::read_to_string(&path).map_err(|err|
        Error::io("Failed to read TOML file", &path, err)
    )?;

    toml::from_str(&text).map_err(|err|
        Error::invalid_toml(path, err)
    )
}

pub fn write_toml_file<T: serde::ser::Serialize>(value: &T, path: impl AsRef<Path>) -> Result<(), Error> {
    let toml_text = toml::to_string(value).map_err(|err|
        Error::toml_ser(type_name::<T>(), err)
    )?;

    fs::write(&path, toml_text).map_err(|err|
        Error::io("Failed to write TOML file", path, err)
    )
}
