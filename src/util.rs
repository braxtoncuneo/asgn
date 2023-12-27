use dirs;

use std::
{
    ffi::{
        OsString,
        OsStr,
    },
    fs::
    {
        self,
        OpenOptions,
    },
    io::Write,
    os::unix::
    ffi::OsStringExt,
    path::{
        Path,
        PathBuf,
    },
    process::
    {
        Command,
        ExitStatus,
        Stdio,
    },
};


use crate::fail_info::
{
    FailInfo,
    FailLog,
};

use toml;
use termion::terminal_size;
use colored::Colorize;
use walkdir::WalkDir;
use chrono::{
    TimeZone,
    Datelike,
    Timelike,
};

pub fn print_hline () {
    if let Ok((w,_h)) = terminal_size() {
        println!("{:-<1$}","",w as usize);
    } else {
        println!("");
    }
}


pub fn print_bold_hline () {
    if let Ok((w,_h)) = terminal_size() {
        println!("{}",format!("{:=<1$}","",w as usize).bold());
    } else {
        println!("");
    }
}



pub fn run_at
<P : AsRef<Path> >
(mut cmd: Command, path: P, pipe_stdout : bool) -> Result<(ExitStatus,OsString,OsString),FailLog>
{
    let cmd_str = format!("{:?}",&cmd);

    let output = if pipe_stdout {
        cmd
        .current_dir(path.as_ref())
        .stdout(Stdio::inherit())
        .spawn()
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("running command {} : {}",cmd_str,err)).into()
        })?
        .wait_with_output()
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("running command {} : {}",cmd_str,err)).into()
        })?
    } else {
        cmd
        .current_dir(path.as_ref())
        .output()
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("running command {} : {}",cmd_str,err)).into()
        })?
    };

    let stderr = OsString::from_vec(output.stderr);
    let stdout = OsString::from_vec(output.stdout);
    let status = output.status;

    Ok((status,stdout,stderr))
}



pub fn set_mode
<P : AsRef<Path> >
(path: P, mode: u32) -> Result<(),FailLog>
{

    let mut cmd = std::process::Command::new("chmod");
    cmd.arg(format!("{:o}",mode));
    cmd.arg(path.as_ref());

    let output = cmd.output()
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("chmoding {} : {}",path.as_ref().display(),err)).into()
        })?;

    let stderr = OsString::from_vec(output.stderr);
    if ! output.status.success() {
        return Err(FailInfo::IOFail(stderr.to_string_lossy().to_string()).into())
    }

    Ok(())
}

#[derive(Clone)]
pub struct FaclEntry
{
    pub user  : OsString,
    pub read  : bool,
    pub write : bool,
    pub exe   : bool,
}

impl ToString for FaclEntry{
    fn to_string(&self) -> String
    {
        let user  = self.user.to_string_lossy().to_string();
        let read  = if self.read  { "r" } else { "" };
        let write = if self.write { "w" } else { "" };
        let exe   = if self.exe   { "x" } else { "" };

        user + ":" + read + write + exe
    }
}


pub fn set_facl
<'facl, P: AsRef<Path>, E: Iterator<Item=&'facl FaclEntry>>
(path : P, default: bool, mut entries: E) -> Result<(),FailLog>
{
    let Some(mut entry_string) = entries.next()
        .map(ToString::to_string) else {
            return Ok(())
        };

    for entry in entries {
        entry_string.push(',');
        entry_string += &entry.to_string();
    }

    let mut cmd = std::process::Command::new("setfacl");

    if default {
        cmd.arg("-d");
    }

    cmd.arg("-m");

    cmd.arg(entry_string).arg(path.as_ref());

    let output = cmd.output()
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("fusing setfacl : {}",err)).into()
        })?;

    let stderr = OsString::from_vec(output.stderr);
    if ! output.status.success() {
        return Err(FailInfo::IOFail(stderr.to_string_lossy().to_string()).into())
    }

    Ok(())
}

pub fn write_file
<P : AsRef<Path>, S : AsRef<[u8]>>
( path: P, slice: S)
-> Result<(),FailLog>
{
    let path_ref : &Path = path.as_ref();
    fs::write(path_ref,slice)
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!(
                "writing contents of {} : {}",
                path_ref.display(),err
            )).into()
        })
}


pub fn refresh_file
<P : AsRef<Path>>
(
    path: P,
    mode: u32,
    default_text: String
) -> Result<(),FailLog>
{
    let path = path.as_ref();

    /*
    if path.exists() && path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("removing directory {} : {}",path.display(),err)).into()
            })?;
    }
    */

    if ! path.exists() {
        fs::write(path,default_text)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("creating default file for {} : {}", path.display(),err)).into()
            })?;
    }

    set_mode(path,mode)?;

    Ok(())
}


pub fn refresh_dir
<'facl, P: AsRef<Path>, E: Iterator<Item=&'facl FaclEntry> + Clone >
(
    path: P,
    mode: u32,
    facl: E,
) -> Result<(),FailLog>
{

    let path = path.as_ref();

    /*
    if path.exists() && path.is_file() {
        fs::remove_file(path)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("removing file {} : {}",path.display(),err)).into()
            })?;
    }
    */

    if ! path.exists() {
        fs::create_dir(path)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("creating directory {} : {}", path.display(),err)).into()
            })?;
    }

    set_mode(path,mode)?;

    set_facl(path,false,facl.clone())?;
    set_facl(path,true, facl)?;

    Ok(())

}


pub fn recursive_refresh_dir
<'facl, P: AsRef<Path> + Clone, E: Iterator<Item=&'facl FaclEntry> + Clone >
(
    path: P,
    mode: u32,
    facl: E,
) -> Result<(),FailLog>
{
    if ! path.as_ref().exists() {
        refresh_dir(path.clone(),mode,facl.clone())?;
        return Ok(());
    }

    if path.as_ref().is_file() {
        return refresh_file(path.clone(),mode,String::new());
    } else if path.as_ref().is_dir() {
        refresh_dir(path.clone(),mode,facl.clone())?;
    } else {
        return Ok(());
    }

    for maybe_entry in WalkDir::new(path).min_depth(1)
    {
        let  dir_entry = maybe_entry.map_err(|err| FailInfo::IOFail(format!("{}",err)).into_log())?;
        recursive_refresh_dir(dir_entry.path(),mode,facl.clone())?;
    }
    Ok(())
}

pub fn bashrc_append_line
<L : AsRef<str> + std::fmt::Display>
(line : L) -> Result<(),FailLog> {
    use FailInfo::*;
    let home_path   : PathBuf = dirs::home_dir()
        .ok_or(IOFail("Home directory cannot be determined".to_string()).into_log())?;
    let bashrc_path : PathBuf = home_path.join(".bashrc");
    let mut bashrc = OpenOptions::new()
        .write(true)
        .append(true)
        .open(bashrc_path)
        .map_err(|err| IOFail(format!("{}",err)))?;
    writeln!(bashrc,"{}",line)
        .map_err(|err| IOFail(format!("{}",err)))?;
    Ok(())
}


pub fn stringify_osstr_vec(vec: &Vec<OsString>) -> Vec<String> {
    vec.iter()
        .map(|os_str| -> String {
            OsStr::to_string_lossy(os_str).to_string()
        })
        .collect()
}

pub fn osify_string_vec(vec: &Vec<String>) -> Vec<OsString> {
    vec.iter()
        .map(|string| -> OsString {
            OsString::from(string)
        })
        .collect()
}


pub fn make_fresh_dir(path : &Path, base_name : &str)
    -> PathBuf
{
    let mut idx : Option<usize> = None;

    let gen_path = |idx: Option<usize>| {
        if let Some(idx) =idx {
            path.join(format!("{}.{}",base_name,idx))
        } else {
            path.join(base_name)
        }
    };

    while gen_path(idx).exists() {
        idx = idx.map(|i|i+1).or(Some(0));
    }
    gen_path(idx)
}



pub fn date_into_chrono(deadline : toml::value::Datetime) -> Result<chrono::DateTime<chrono::Local>,FailLog> {
    let (hr,min,sec) : (u32,u32,u32) = if let Some (time) = deadline.time {
        (time.hour.into(),time.minute.into(),time.second.into())
    } else {
        (23,59,59)
    };

    let result = match deadline.date {
        Some(date) => {
            let y : i32 = date.year  as i32;
            let m : u32 = date.month as u32;
            let d : u32 = date.day   as u32;
            Ok(chrono::offset::Local.with_ymd_and_hms(y,m,d,hr,min,sec).unwrap())
        },
        None       => {
            Err(FailInfo::BadSpec(
                "assignment".to_string(),
                String::from("Date data missing from deadline field.")
            ).into_log())
        },
    };
    let result = result?;
    //println!("-> {} <- ",result);
    Ok(result)
}



pub fn date_from_chrono(deadline : chrono::DateTime<chrono::Local>) -> toml::value::Datetime{
    let date_naive = deadline.date_naive();
    let time = deadline.time();
    toml::value::Datetime {
        date: Some(toml::value::Date{
            year  : date_naive.year()  as u16,
            month : date_naive.month() as u8,
            day   : date_naive.day()   as u8,
        }),
        time: Some(toml::value::Time{
            hour   : time.hour()   as u8,
            minute : time.minute() as u8,
            second : time.second() as u8,
            nanosecond : 0,
        }),
        offset: None,
    }
}

pub fn parse_from<T : serde::de::DeserializeOwned > (path : &Path) -> Result<T,FailLog> {
    let text = fs::read_to_string(path)
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("reading file : {}",err)).into()
        })?;
    let result = toml::from_str(&text)
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("deserializing file : {}",err)).into()
        });
    result
}


pub fn serialize_into<T : serde::ser::Serialize > (path : &Path, value : &T) -> Result<(),FailLog> {
    let toml_text  = toml::to_string(value)
        .map_err( |err| -> FailLog {
            FailInfo::IOFail(format!("serializing extension file : {}",err)).into()
        })?;
    fs::write(path,toml_text)
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("writing extension file : {}",err)).into()
        })
}

