
use std::
{
    collections::HashMap,
    env::current_dir,
    ffi::
    {
        OsString,
        OsStr,
    },
    fs,
    io::
    {
        ErrorKind,
        Write,
    },
    os::
    {
        unix::fs::
        {
            MetadataExt,
            DirBuilderExt,
            PermissionsExt,
        }
    },
    path::
    {
        Path,
        PathBuf,
    }
};

use chrono::
{
    DateTime,
    Local,
};

use users::
{
    get_user_by_uid,
    get_current_uid,
};

use crate::{
    fail_info::
    {
        FailInfo,
        FailLog,
    },
    asgn_spec::AsgnSpec,
};


use serde_derive::
{
    Serialize,
    Deserialize,
};


#[derive(Default,Serialize,Deserialize)]
struct CourseToml
{
    manifest   : Vec<String>,
    graders    : Vec<String>,
    students   : Vec<String>,
    style_path : String,
}



pub struct Context
{
    // Determined through input
    pub course      : OsString,
    pub instructor  : OsString,
    pub base_path   : PathBuf,

    // Determined through system calls
    pub uid         : u32,
    pub user        : OsString,
    pub time        : DateTime<Local>,
    pub cwd         : PathBuf,

    // Determined by reading the context file
    pub manifest    : Vec<OsString>,
    pub graders     : Vec<OsString>,
    pub students    : Vec<OsString>,

    // Determined by trying to parse the spec of every
    // assignment in the manifest
    pub catalog     : HashMap<OsString,Result<AsgnSpec,FailLog>>,
}


impl Context
{

    fn load_course_spec(base_path: &PathBuf) -> Result<CourseToml,FailLog>
    {
        let course_file_path = base_path.join(".course.toml");

        let course_file_text = fs::read_to_string(course_file_path)
            .map_err(|err| -> FailLog {
                FailInfo::NoSpec(
                    "course".to_string(),
                    format!("{}",err)
                ).into()
            })?;

        toml::from_str(&course_file_text)
            .map_err(|err| -> FailLog {
                FailInfo::BadSpec(
                    "course".to_string(),
                    format!("{}",err)
                ).into()
            })


    }


    fn populate_catalog(&mut self)
    {
        for asgn_name in self.manifest.iter() {
            let spec_path = self.base_path
                .join(asgn_name);

            self.catalog.insert(asgn_name.clone(), AsgnSpec::new(spec_path));
        }
    }

    pub fn deduce(instructor: OsString, course: OsString) -> Result<Self,FailLog>
    {
        let uid  : u32      = get_current_uid();
        let user : OsString = get_user_by_uid(uid)
                .ok_or(FailInfo::InvalidUID() )?.name().into();

        let cwd = current_dir()
                .map_err(|_| FailInfo::InvalidCWD() )?;

        let base_path = PathBuf::from("/home/fac")
                .join(&instructor)
                .join("submit")
                .join(&course);

        if ! base_path.is_dir() {
            return Err(FailInfo::NoBaseDir(base_path).into());
        }

        let time = Local::now();

        let spec = Self::load_course_spec(&base_path)?;
        let manifest : Vec<OsString> = spec.manifest.into_iter().map(OsString::from).collect();
        let graders  : Vec<OsString> = spec.graders .into_iter().map(OsString::from).collect();
        let students : Vec<OsString> = spec.students.into_iter().map(OsString::from).collect();

        let mut context = Self {
            course,
            instructor,
            base_path,
            uid,
            user,
            time,
            cwd,
            manifest,
            graders,
            students,
            catalog : Default::default(),
        };

        context.populate_catalog();
        Ok(context)
    }


    pub fn print_manifest(&self)
    {
        for name in self.manifest.iter()
        {
            println!("{}",name.to_string_lossy());
        }
    }

    pub fn print_failures(&self)
    {
        let mut log : FailLog = Default::default();
        for sub_log in self.manifest.iter()
            .flat_map(|name| self.catalog[name].as_ref().err())
        {
            log.extend((*sub_log).clone());
        }
        print!("{}",log);

    }


    pub fn get_spec<'a> (manifest : &'a Vec<AsgnSpec>, name: &OsStr) -> Option<&'a AsgnSpec>
    {
        for entry in manifest.iter() {
            if OsString::from(&entry.name) == name {
                return Some(entry);
            }
        }
        None
    }


    pub fn announce(&self)
    {
        println!("The time is : {}",self.time);
        println!("The user is : {}",self.user.to_string_lossy());
        println!("Called from directory {}",self.cwd.display());       
    }



    fn make_dir_public<P: AsRef<Path>, L : AsRef<str>>(path: P, label: L) -> Result<(),FailLog>
    {
        let mut perm = fs::metadata(path.as_ref())
            .map_err(|err| -> FailLog {
                FailInfo::IOFail(format!("chmoding {} : {}",label.as_ref(),err)).into()
            })?.permissions();
        
        perm.set_mode(0o755);
        Ok(())
    }


    pub fn refresh(&self) -> Result<(),FailLog>
    {

        // Make sure the base directory has the correct permissions
        Context::make_dir_public(&self.base_path,"course directory")?;


        let mut dir_builder = fs::DirBuilder::new();
        
        // Make sure all of the assignments have a directory with the correct permissions
        for name in self.manifest.iter() {
            let asgn_path = self.base_path.join(name);
            dir_builder.create(&self.base_path)
                .map_err(|err| -> FailLog {
                    FailInfo::IOFail(format!("creating {} assignment directory : {}",name.to_string_lossy(),err)).into()
                })?;
            Context::make_dir_public(&self.base_path,format!("{} assignment directory",name.to_string_lossy()))?;
        }

        // Make sure all of the assignment directories have a subdirectory for each student,
        // again - with the correct permissions


        
        // Make sure every grader has a grading directory with the correct permissions


        Ok(())
    }


    /*
    pub fn refresh(&self) -> Result<(),FailLog>
    {
        let mut dir_builder = fs::DirBuilder::new();
        dir_builder.create(&self.base_path)
            .map_err(|err|{
                FailInfo::IOFail(format!("creating base directory : {}",err))
            })?;
        
        let mut perm = fs::metadata(&self.base_path)
            .map_err(|err| {
                FailInfo::IOFail(format!("opening base directory : {}",err))
            })?.permissions();
        
        perm.set_mode(0o755);
        
        let course_file_path = self.base_path.join(".course.toml");
        let course_file = fs::File::open(&course_file_path)
            .or_else (|err| -> Result<fs::File,FailLog> {
                match err.kind() {
                    ErrorKind::NotFound => {
                        let mut file = fs::File::create(&course_file_path)
                            .map_err(|inner_err| -> FailLog {
                                FailInfo::IOFail(format!("creating course file : {}",inner_err)).into()
                            })?;
                        let course_toml : CourseToml = Default::default();
                        let course_text = toml::to_string(&course_toml).unwrap();
                        file.write_all(course_text.as_bytes())
                            .map_err(|inner_err| -> FailLog {
                                FailInfo::IOFail(format!("initializing course file : {}",inner_err)).into()
                            })?;
                        Ok(file)
                    },
                    x => Err(FailInfo::IOFail(format!("opening course file : {}",x)).into()),
                }
            })?;
        

        todo!()

    }
    */


}


