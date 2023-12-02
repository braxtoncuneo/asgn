
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
    os::unix::fs::{
        PermissionsExt,
        MetadataExt,
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
    asgn_spec::
    {
        AsgnSpec,
        AsgnSpecToml,
    },
    util,
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
}


pub enum Role
{
    Instructor,
    Grader,
    Student,
    Other,
}


pub struct Context
{
    // Determined through input
    pub instructor  : OsString,
    pub base_path   : PathBuf,
    pub exe_path    : PathBuf,

    // Determined through system calls
    pub uid         : u32,
    pub user        : OsString,
    pub time        : DateTime<Local>,
    pub cwd         : PathBuf,

    // Determined by reading the context file
    pub manifest    : Vec<OsString>,
    pub graders     : Vec<OsString>,
    pub students    : Vec<OsString>,

    // Determined by the context file + system calls
    pub role        : Role,

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

    pub fn sync_course_spec(&self) -> Result<(),FailLog>
    {
        use FailInfo::*;
        let course_file_path = self.base_path.join(".course.toml");

        let convert = |vec : &Vec<OsString> | -> Vec<String> {
            vec.iter()
                .map(|os_str| -> String {
                    OsStr::to_string_lossy(os_str).to_string()
                })
                .collect()
        };

        let course_toml = CourseToml {
            manifest: convert(&self.manifest),
            graders:  convert(&self.graders),
            students: convert(&self.students)
        };
    
        let toml_text = toml::to_string(&course_toml)
            .map_err(|err| IOFail(format!(
                "Could not serialize course spec : {}",
                err
            )).into_log())?;

        util::write_file(
            course_file_path,
            toml_text
        )
    }


    fn populate_catalog(&mut self)
    {
        for asgn_name in self.manifest.iter() {
            let spec_path = self.base_path
                .join(asgn_name);

            self.catalog.insert(asgn_name.clone(), AsgnSpec::new(spec_path));
        }
    }

    pub fn deduce(base_path: OsString) -> Result<Self,FailLog>
    {
        let base_path : PathBuf = base_path.into();
        let uid  : u32      = get_current_uid();
        let user : OsString = get_user_by_uid(uid)
                .ok_or(FailInfo::InvalidUID() )?.name().into();

        let cwd = current_dir()
                .map_err(|_| FailInfo::InvalidCWD() )?;
        
        let exe_path = std::fs::read_link("/proc/self/exe")
            .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;

        if ! base_path.is_dir() {
            return Err(FailInfo::NoBaseDir(base_path).into());
        }

        let instructor_uid = std::fs::metadata(&base_path)
            .map_err(|err| FailInfo::IOFail(format!("{}",err)))?.uid();
        let instructor : OsString = get_user_by_uid(instructor_uid)
                .ok_or(FailInfo::InvalidUID() )?.name().into();

        let time = Local::now();

        let spec = Self::load_course_spec(&base_path)?;
        let manifest : Vec<OsString> = spec.manifest.into_iter().map(OsString::from).collect();
        let graders  : Vec<OsString> = spec.graders .into_iter().map(OsString::from).collect();
        let students : Vec<OsString> = spec.students.into_iter().map(OsString::from).collect();

        let mut role = Role::Other;

        if students.iter().any(|s| *s == user ) {
            role = Role::Student;
        }

        if graders.iter().any(|g| *g == user) {
            role = Role::Grader;
        }

        if user == instructor {
            role = Role::Instructor;
        }

        let mut context = Self {
            instructor,
            base_path,
            exe_path,
            uid,
            user,
            time,
            cwd,
            manifest,
            graders,
            students,
            role,
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


    pub fn build_command(&self,spec: &AsgnSpec) -> std::process::Command
    {
        let path = self.base_path.join(&spec.name).join(".spec").join("Makefile");
        let mut cmd  = std::process::Command::new("make");
        cmd.arg(format!("--file={}",path.display()));
        cmd
    }

    pub fn style_command(&self, spec: &AsgnSpec) -> std::process::Command
    {
        let mut cmd = self.build_command(spec);
        cmd.arg("style");
        cmd
    }

    pub fn format_command(&self, spec: &AsgnSpec) -> std::process::Command
    {
        let mut cmd = self.build_command(spec);
        cmd.arg("format");
        cmd
    }

    pub fn test_command(&self, spec: &AsgnSpec) -> std::process::Command
    {
        let mut cmd = self.build_command(spec);
        cmd.arg("test");
        cmd
    }


    pub fn refresh(&self) -> Result<(),FailLog>
    {
        util::refresh_dir(&self.base_path,0o755,Vec::new().iter())?;
        
        let course_file_path = self.base_path.join(".course.toml");
        let course_toml : CourseToml = Default::default();
        let course_text = toml::to_string(&course_toml).unwrap();
        util::refresh_file(&course_file_path,0o644,course_text)?;

        let course_format_path = self.base_path.join(".clang-format");
        let course_style_path  = self.base_path.join(".clang-tidy");
        util::refresh_file(&course_format_path,0o644,"".to_string())?;
        util::refresh_file(&course_style_path ,0o644,"".to_string())?;

        for asgn in self.manifest.iter() {

            let asgn_path = self.base_path.join(asgn);
            util::refresh_dir(&asgn_path,0o755,Vec::new().iter())?;

            let asgn_spec_path = asgn_path.join(".spec");
            util::refresh_dir(&asgn_spec_path,0o755,Vec::new().iter())?;

            let asgn_info_path = asgn_spec_path.join("info.toml");
            let asgn_toml = AsgnSpecToml::default_with_name(asgn);
            let asgn_text = toml::to_string(&asgn_toml).unwrap();
            util::refresh_file(&asgn_info_path,0o644,asgn_text)?;

            let asgn_make_path = asgn_spec_path.join("Makefile");
            let make_text = AsgnSpec::default_makefile(asgn.to_string_lossy());
            util::refresh_file(&asgn_make_path,0o644,make_text)?;

            let _asgn_check_path = asgn_spec_path.join("check");
            util::refresh_dir(&asgn_spec_path,0o755,Vec::new().iter())?;


            for stud in self.students.iter() {
                let asgn_sub_path = asgn_path.join(stud);
                let mut facl_list : Vec<util::FaclEntry> = Vec::new();
                let inst_entry = util::FaclEntry {
                    user  : self.instructor.clone(),
                    read  : true,
                    write : true,
                    exe   : true,
                };

                facl_list.push(inst_entry);
                
                let stud_entry = util::FaclEntry {
                    user  : stud.clone(),
                    read  : true,
                    write : true,
                    exe   : true,
                };

                facl_list.push(stud_entry);
                
                for grad in self.graders.iter () {
                    if grad == &self.instructor  || grad == stud {
                        continue;
                    }
                    let grad_entry = util::FaclEntry {
                        user  : grad.clone(),
                        read  : true,
                        write : false,
                        exe   : true,
                    };
                    facl_list.push(grad_entry);
                }

                util::refresh_dir(asgn_sub_path,0o700,facl_list.iter())?;

            }
        } 

        Ok(())

    }


}


