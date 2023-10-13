
use std::
{
    ffi::
    {
        OsString,
        OsStr,
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
pub struct AsgnSpecToml
{
    name      : String,
    active    : bool,
    visible   : bool,
    deadline  : toml::value::Datetime,
    file_list : Vec<String>,
}

impl Default for AsgnSpecToml
{
    fn default() -> Self
    {
        let date = toml::value::Date {
            year  : 1970,
            month : 1,
            day   : 1,
        };

        let deadline = toml::value::Datetime {
            date   : Some(date),
            time   : None,
            offset : None,
        };

        Self {
            name      : "<put name here>".to_string(),
            active    : false,
            visible   : false,
            deadline,
            file_list : Vec::new(),
        }
    }
}


impl AsgnSpecToml
{
 
    pub fn default_with_name <S: AsRef<OsStr>> (name : S) -> Self {
        let mut result : Self = Default::default();
        result.name = name.as_ref().to_string_lossy().to_string();
        result
    }

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

    pub fn default_makefile <F: std::fmt::Display> (name: F) -> String {
        format!(
           "\nflags =  -Wall -Werror -pedantic -std=c++11\
            \nfile = {name}\
            \n\
            \n$(file): $(file).cpp\
            \n\tg++ -fdiagnostics-color=always $(file).cpp -o $(file) $(flags)\
            \n\
            \nformat: $(file).cpp\
            \n\t@clang-format -i $(file).cpp\
            \n\
            \nstyle: $(file).cpp\
            \n\t@clang-tidy --use-color --fix --quiet $(file).cpp -- $(flags) -include \"iostream\"\
            \n\
            \ntest: $(file).cpp\
            \n\t@echo \"No automated tests provided for this assignment.\""
        )
    }


}


pub struct SubmissionSlot <'ctx>
{
    pub context   : &'ctx Context,
    pub asgn_spec : &'ctx AsgnSpec,
    pub base_path : PathBuf,
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


impl <'ctx> SubmissionSlot<'ctx>
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
            });
        if toml_text.is_err() {
            return Ok(0);
        }
        let bonus : BonusToml = toml::from_str(&toml_text.unwrap())
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("deserializing bonus file : {}",err)).into()
            })?;
        Ok(bonus.value)
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
            })?;
        Ok(())
    }

    pub fn get_extension(&self) -> Result<i64,FailLog>
    {
        let ext_path = self.extension_path();

        if ! ext_path.exists() {
            return Ok(0);
        }
        if ext_path.is_dir() {
            return Ok(0);
        }

        let toml_text = read_to_string(ext_path)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("reading extension file : {}",err)).into()
            })?;
        let ext : ExtensionToml = toml::from_str(&toml_text)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("deserializing extension file : {}",err)).into()
            })?;
        
        Ok(ext.value)
    } 

    pub fn set_extension(&self, value: i64) -> Result<(),FailLog>
    {
        let _ext_path = self.extension_path();

        let ext_toml = ExtensionToml { value };
        let toml_text  = toml::to_string(&ext_toml)
            .map_err( |err| -> FailLog {
                FailInfo::IOFail(format!("serializing extension file : {}",err)).into()
            })?;
        fs::write(self.bonus_path(),toml_text)
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("writing extension file : {}",err)).into()
            })?;
        Ok(())
    }

    pub fn status(&self) -> Result<SubmissionStatus,FailLog>
    {
        let submitted = self.file_paths().into_iter()
            .map(|p| p.exists() && p.is_file() )
            .all(|x| x);

        let time : Option<i64> = if submitted {
            let mut mtime : i64 = 0;
            for path in self.file_paths().into_iter() {
                let meta = fs::metadata(path)
                    .map_err(|err|{
                       FailInfo::IOFail(format!("{}",err.kind())).into_log() 
                    })?;
                mtime = mtime.max(meta.mtime());
            }
            Some(mtime)
        } else {
            None
        };

        let turn_in_time = if let Some(seconds) = time {
            let turn_in = Local.timestamp_opt(seconds,0)
            .earliest()
            .ok_or(FailInfo::IOFail("Impossible time conversion".to_string()).into_log())?;
            Some(turn_in)
        } else {
            None
        };

        Ok(SubmissionStatus {
            turn_in_time,
            bonus_days     : self.get_bonus()?,
            extension_days : self.get_extension()?,
        })
    }


    pub fn refresh(self)
    {
        todo!()
    }

}


impl SubmissionStatus {

    pub fn time_past(&self,time: &DateTime<Local>) -> Option<Duration> {
        let difference = self.turn_in_time?.signed_duration_since(*time);
        return Some(difference)
    }

    pub fn versus(&self,time: &DateTime<Local>) -> String {

        let late_by = self.time_past(time);

        if late_by.is_none() {
            let time_diff = chrono::offset::Local::now().signed_duration_since(*time);
            if time_diff.num_seconds() <= 0 {
                return String::from("Not Submitted");
            } else {
                return String::from("Missing");
            }
        }

        let late_by = late_by.unwrap();

        let mut total : i64 = 0;
        let days : i64 = late_by.num_days();
        total += days;
        total *= 24;
        let hours : i64 = late_by.num_hours() - total;
        total += hours;
        total *= 60;
        let mins : i64 = late_by.num_minutes() - total;
        
        if late_by.num_seconds() > 0 {
            format!("Late {}d {}h {}m",days,hours,mins)
        } else {
            format!("Early {}d {}h {}m",-days,-hours,-mins)
        }
    }


}

