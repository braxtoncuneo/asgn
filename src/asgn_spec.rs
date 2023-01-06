
use std::
{
    ffi::
    {
        OsString,
    },
    fs::{
        self,
        read_to_string,
    },
    path::
    {
        PathBuf,
    },
    os::unix::fs::MetadataExt,
};


use serde_derive::
{
    Serialize,
    Deserialize,
};

use chrono::
{
    DateTime,
    Local,
    TimeZone,
    Duration,
};


use toml;

use crate::{
    fail_info::{
        FailInfo,
        FailLog,
    },
    context::Context,
};


#[derive(Serialize,Deserialize)]
struct AsgnSpecToml
{
    name      : String,
    active    : bool,
    visible   : bool,
    deadline  : toml::value::Datetime,
    file_list : Vec<String>,
}





pub struct AsgnSpec
{
    pub name      : String,
    pub active    : bool,
    pub visible   : bool,
    pub deadline  : DateTime<Local>,
    pub file_list : Vec<OsString>,
}


impl TryFrom<AsgnSpecToml> for AsgnSpec
{
    type Error = FailLog;

    fn try_from(spec_toml: AsgnSpecToml) -> Result<Self,Self::Error> {
        let deadline = match spec_toml.deadline.date {
            Some(date) => {
                let y : i32 = date.year  as i32;
                let m : u32 = date.month as u32;
                let d : u32 = date.day   as u32;
                chrono::offset::Local.with_ymd_and_hms(y,m,d,23,59,59).unwrap()
            },
            None       => {
                return Err(FailInfo::BadSpec(
                    "assignment".to_string(),
                    String::from("Date data missing from deadline field.")
                ).into())
            },
        };
        let mut file_list = Vec::<OsString>::new();
        for filename in spec_toml.file_list.iter()
        {
            file_list.push(OsString::from(filename));
        }

        Ok(Self {
            name     : spec_toml.name,
            active   : spec_toml.active,
            visible  : spec_toml.visible,
            deadline,
            file_list,
        })

    }
}


impl AsgnSpec
{

    pub fn new(path : PathBuf) -> Result<Self,FailLog>
    {

        let spec_path = path.join(".spec");
        let info_path = spec_path.join("info.toml");

        //println!("{}",info_path.display());

        let info_text = read_to_string(info_path)
            .map_err(|err|  -> FailLog {
                FailInfo::NoSpec(
                    "assignment".to_string(),
                    format!("{}",err)
                ).into()
            })?;

        let info_toml : AsgnSpecToml = toml::from_str(&info_text)
            .map_err(|err| -> FailLog {
                FailInfo::BadSpec(
                    "assignment".to_string(),
                    format!("{}",err)
                ).into()
            })?;

        info_toml.try_into()
            .and_then(|spec : AsgnSpec | -> Result<Self,FailLog> {
                if ! path.ends_with(&spec.name) {
                    Err(FailInfo::BadSpec(
                        "assignment".to_string(),
                        String::from("Name field does not match assignment directory name.")
                    ).into())
                } else {
                    Ok(spec)
                }
            })

    }


}


pub struct SubmissionDir <'ctx>
{
    context   : &'ctx Context,
    asgn_spec : &'ctx AsgnSpec,
    base_path : PathBuf,
}

pub struct SubmissionStatus
{
    pub turn_in_time   : Option<DateTime<Local>>,
    pub bonus_days     : i64,
    pub extension_days : i64,
}


#[derive(Serialize,Deserialize)]
struct BonusToml
{
    pub value : i64,
}


#[derive(Serialize,Deserialize)]
struct ExtensionToml
{
    pub value : i64,
}


impl <'ctx> SubmissionDir<'ctx>
{

    pub fn bonus_path(&self) -> PathBuf
    {
        self.base_path.join(".bonus")
    }

    pub fn extension_path(&self) -> PathBuf
    {
        self.base_path.join(".extension")
    }

    pub fn file_paths(&self) -> Vec<PathBuf>
    {
        self.asgn_spec.file_list.iter()
            .map(|name| self.base_path.join(name))
            .collect()
    }

    pub fn get_bonus(&self) -> Result<i64,FailLog>
    {
        let toml_text = read_to_string(self.bonus_path())
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("reading bonus file : {}",err)).into()
            })?;
        toml::from_str(&toml_text)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("deserializing bonus file : {}",err)).into()
            })
    }

    pub fn set_bonus(&self, value: i64) -> Result<(),FailLog>
    {
        let bonus_toml = BonusToml { value };
        let toml_text  = toml::to_string(&bonus_toml)
            .map_err( |err| -> FailLog {
                FailInfo::IOFail(format!("serializing bonus file : {}",err)).into()
            })?;
        fs::write(self.bonus_path(),toml_text)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("writing bonus file : {}",err)).into()
            })
    }

    pub fn get_extension(&self) -> Result<Option<u8>,FailLog>
    {
        todo!()
    } 

    pub fn set_extension(&self) -> Result<(),FailLog>
    {
        todo!()
    }

    pub fn status(&self) -> SubmissionStatus
    {
        todo!()
    }

    pub fn copy_into_sub(&self, source_path: PathBuf)
    {
        todo!()
    }

    pub fn copy_from_sub(&self, destination_path: PathBuf)
    {
        todo!()
    }

    pub fn refresh(self)
    {
        todo!()
    }

}

