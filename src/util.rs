
use std::
{
    ffi::OsString,
    fs::
    {
        self,
    },
    os::unix::
    {
        fs::PermissionsExt,
        ffi::OsStringExt,
    },
    path::Path,
    process::
    {
        Command,
        ExitStatus,
    },
};


use crate::
{
    fail_info::
    {
        FailInfo,
        FailLog,
    }
};


pub fn run_at
<P : AsRef<Path> >
(mut cmd: Command, path: P) -> Result<(ExitStatus,OsString,OsString),FailLog>
{
    let cmd_str = format!("{:?}",&cmd);

    let output = cmd
        .current_dir(path.as_ref())
        .output()
        .map_err(|err| -> FailLog {
            FailInfo::IOFail(format!("running command {} : {}",cmd_str,err)).into()
        })?;

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
            FailInfo::IOFail(format!("running chmod: {}",err)).into()
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


pub fn refresh_file
<P : AsRef<Path>>
(
    path: P,
    mode: u32,
    default_text: String
) -> Result<(),FailLog>
{
    let path = path.as_ref();

    if path.exists() && path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("removing directory {} : {}",path.display(),err)).into()
            })?;
    }
    
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

    if path.exists() && path.is_file() {
        fs::remove_file(path)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("removing file {} : {}",path.display(),err)).into()
            })?;
    }

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






