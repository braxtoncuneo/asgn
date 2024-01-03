use std::{
    fmt::{self, Write as _},
    fs::{self, Permissions},
    io::Write as _,
    path::{Path, PathBuf},
    os::unix::fs::PermissionsExt,
};

use itertools::Itertools;
use walkdir::WalkDir;

use crate::error::Error;

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
