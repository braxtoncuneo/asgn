use dirs;
use itertools::Itertools;

use std::{
    fs::{self, Permissions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio}, fmt::{self, Write as _},
    os::unix::fs::PermissionsExt,
};

use crate::error::Error;

use toml;
use termion::terminal_size;
use walkdir::WalkDir;
use chrono::{TimeZone, Datelike, Timelike};

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

    let output = if pipe_stdout {
        cmd
            .stdout(Stdio::inherit())
            .spawn()
            .map_err(|err| Error::IOFail(format!("running command {cmd:?}: {err}")))?
            .wait_with_output()
            .map_err(|err| Error::IOFail(format!("running command {cmd:?}: {err}")))?
    } else {
        cmd
            .output()
            .map_err(|err| Error::IOFail(format!("running command {cmd:?}: {err}")))?
    };

    let status = output.status;

    Ok(status)
}

pub fn set_mode(path: impl AsRef<Path>, mode: u32) -> Result<(), Error> {
    fs::set_permissions(path.as_ref(), Permissions::from_mode(mode)).map_err(|err|
        Error::IOFail(format!("chmoding {}: {}", path.as_ref().display(), err))
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
        Error::IOFail(format!("fusing setfacl: {err}"))
    )?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(Error::IOFail(err_msg))
    }

    Ok(())
}

pub fn write_file(path: impl AsRef<Path>, slice: impl AsRef<[u8]>) -> Result<(), Error> {
    let path = path.as_ref();
    fs::write(path, slice).map_err(|err|
        Error::IOFail(format!(
            "writing contents of {}: {}",
            path.display(), err
        ))
    )
}

pub fn refresh_file(path: impl AsRef<Path>, mode: u32, default_text: &str) -> Result<(), Error> {
    let path = path.as_ref();

    if !path.exists() {
        fs::write(path, default_text).map_err(|err|
            Error::IOFail(format!("creating default file for {}: {}", path.display(), err))
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
            Error::IOFail(format!("creating directory {}: {}", path.display(), err))
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
            Error::IOFail(err.to_string())
        )?;
        recursive_refresh_dir(dir_entry.path(), mode, facl.clone())?;
    }

    Ok(())
}

pub fn bashrc_append_line(line: &str) -> Result<(), Error> {
    let home_path: PathBuf = dirs::home_dir().ok_or_else(||
        Error::IOFail("Home directory cannot be determined".to_owned())
    )?;

    let bashrc_path : PathBuf = home_path.join(".bashrc");
    let mut bashrc = fs::OpenOptions::new()
        .append(true)
        .open(bashrc_path)
        .map_err(|err| Error::IOFail(err.to_string()))?;

    writeln!(bashrc, "{line}").map_err(|err|
        Error::IOFail(err.to_string())
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

pub fn date_into_chrono(deadline: toml::value::Datetime) -> Result<chrono::DateTime<chrono::Local>, Error> {
    let (hr, min, sec): (u32, u32, u32) =
        if let Some(time) = deadline.time {
            (time.hour.into(), time.minute.into(), time.second.into())
        } else {
            (23, 59, 59)
        };

    match deadline.date {
        Some(date) => {
            let y: i32 = date.year  as i32;
            let m: u32 = date.month as u32;
            let d: u32 = date.day   as u32;
            Ok(chrono::offset::Local.with_ymd_and_hms(y, m, d, hr, min, sec).unwrap())
        },
        None => Err(Error::BadSpec(
            "assignment".to_owned(),
            String::from("Date data missing from deadline field.")
        )),
    }
}

pub fn date_from_chrono(deadline : chrono::DateTime<chrono::Local>) -> toml::value::Datetime {
    let date_naive = deadline.date_naive();
    let time = deadline.time();

    toml::value::Datetime {
        date: Some(toml::value::Date {
            year:  date_naive.year()  as u16,
            month: date_naive.month() as u8,
            day:   date_naive.day()   as u8,
        }),
        time: Some(toml::value::Time {
            hour:   time.hour()   as u8,
            minute: time.minute() as u8,
            second: time.second() as u8,
            nanosecond : 0,
        }),
        offset: None,
    }
}

pub fn parse_from<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let text = fs::read_to_string(path).map_err(|err|
        Error::IOFail(format!("reading file: {err}"))
    )?;

    toml::from_str(&text).map_err(|err|
        Error::IOFail(format!("deserializing file: {err}"))
    )
}

pub fn serialize_into<T: serde::ser::Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    let toml_text  = toml::to_string(value).map_err( |err|
        Error::IOFail(format!("serializing extension file: {err}"))
    )?;

    fs::write(path, toml_text).map_err(|err|
        Error::IOFail(format!("writing extension file: {err}"))
    )
}
