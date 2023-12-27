
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
    TimeZone,
    DateTime,
    Local,
    Days,
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
        SubmissionSlot,
    },
    util,
    table::Table,
    act::instructor::InstructorAct,
};


use serde_derive::
{
    Serialize,
    Deserialize,
};

use itertools::Itertools;


#[derive(Default,Serialize,Deserialize)]
pub struct CourseToml
{
    manifest    : Vec<String>,
    graders     : Vec<String>,
    students    : Vec<String>,
    grace_total : Option<i64>,
    grace_limit : Option<i64>,
}

#[derive(PartialEq)]
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
    pub manifest     : Vec<OsString>,
    pub graders      : Vec<OsString>,
    pub students     : Vec<OsString>,
    pub members      : Vec<OsString>,
    pub grace_total  : Option<i64>,
    pub grace_limit  : Option<i64>,

    // Determined by the context file + system calls
    pub role        : Role,

    // Determined by trying to parse the spec of every
    // assignment in the manifest
    pub catalog     : HashMap<OsString,Result<AsgnSpec,FailLog>>,
}


impl Context
{

    fn load(base_path: &PathBuf) -> Result<CourseToml,FailLog>
    {
        let course_file_path = base_path.join(".info").join("course.toml");

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

    pub fn sync(&self) -> Result<(),FailLog>
    {
        use FailInfo::*;
        let course_file_path = self.base_path.join(".info").join("course.toml");


        let course_toml = CourseToml {
            manifest: util::stringify_osstr_vec(&self.manifest),
            graders:  util::stringify_osstr_vec(&self.graders),
            students: util::stringify_osstr_vec(&self.students),
            grace_total : self.grace_total,
            grace_limit : self.grace_limit,
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


    pub fn populate_catalog(&mut self)
    {
        for asgn_name in self.manifest.iter() {
            let spec_path = self.base_path
                .join(asgn_name);
            self.catalog.insert(asgn_name.clone(), AsgnSpec::load(spec_path));
        }
    }

    pub fn deduce(base_path: OsString) -> Result<Self,FailLog>
    {
        let base_path : PathBuf = base_path.into();
        let base_path = base_path.canonicalize().unwrap();

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

        let spec = Self::load(&base_path)?;
        let manifest : Vec<OsString> = spec.manifest.into_iter().map(OsString::from).collect();
        let graders  : Vec<OsString> = spec.graders .into_iter().map(OsString::from).collect();
        let students : Vec<OsString> = spec.students.into_iter().map(OsString::from).collect();
        let grace_total = spec.grace_total;
        let grace_limit = spec.grace_limit;

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

        let mut members = vec![instructor.clone()];
        members.extend(graders.clone());
        members.extend(students.clone());

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
            members,
            grace_total,
            grace_limit,
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

    pub fn grader_facl(&self,student : Option<&OsString>) -> Result<Vec<util::FaclEntry>,FailLog>
    {
        let mut facl_list : Vec<util::FaclEntry> = Vec::new();
        let inst_entry = util::FaclEntry {
            user  : self.instructor.clone(),
            read  : true,
            write : true,
            exe   : true,
        };

        facl_list.push(inst_entry);

        if let Some(student_name) = student {
            let student_entry = util::FaclEntry {
                user  : student_name.clone(),
                read  : true,
                write : true,
                exe   : true,
            };

            facl_list.push(student_entry);
        }

        for grad in self.graders.iter () {
            let is_instructor = grad == &self.instructor;
            let is_student   = student.map(|name| name == grad).unwrap_or(false);
            if is_student || is_instructor {
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
        Ok(facl_list)
    }


    fn refresh_root(&self) -> Result<(),FailLog>
    {
        util::refresh_dir(&self.base_path,0o755,Vec::new().iter())?;

        let course_info_path = self.base_path.join(".info");
        util::refresh_dir(&course_info_path,0o755,Vec::new().iter())?;

        let course_file_path = course_info_path.join("course.toml");
        let course_toml : CourseToml = Default::default();
        let course_text = toml::to_string(&course_toml).unwrap();
        util::refresh_file(&course_file_path,0o644,course_text)?;

        let empty = Vec::new();
        let grade = self.grader_facl(None)?;
        let dirs = [
            ("public",0o755,empty.iter()),
            ("private",0o700,grade.iter())
        ];

        for (name,flags,facl) in dirs.iter() {
            let path = course_info_path.join(name);
            util::recursive_refresh_dir(&path,*flags,facl.clone())?;
        }
        Ok(())
    }


    pub fn refresh_assignment(&self, asgn: &OsStr) -> Result<(),FailLog>
    {
        let asgn_path = self.base_path.join(asgn);
        util::refresh_dir(&asgn_path,0o755,Vec::new().iter())?;

        let asgn_spec_path = asgn_path.join(".info");
        util::refresh_dir(&asgn_spec_path,0o755,Vec::new().iter())?;

        let asgn_info_path = asgn_spec_path.join("info.toml");
        let asgn_toml = AsgnSpecToml::default_with_name(asgn);
        let asgn_text = toml::to_string(&asgn_toml).unwrap();
        util::refresh_file(&asgn_info_path,0o644,asgn_text)?;

        let asgn_make_path = asgn_spec_path.join("Makefile");
        util::refresh_file(&asgn_make_path,0o644,String::new())?;

        let asgn_make_path = asgn_spec_path.join("score.toml");
        util::refresh_file(&asgn_make_path,0o644,String::new())?;

        let empty = Vec::new();
        let grade = self.grader_facl(None)?;
        let dirs = [
            ("public", 0o755,empty.iter()),
            ("private",0o700,grade.iter()),
        ];

        for (name,flags,facl) in dirs.iter() {
            let path = asgn_spec_path.join(name);
            util::recursive_refresh_dir(&path,*flags,facl.clone())?;
        }

        let internal_path = asgn_spec_path.join(".internal");
        util::recursive_refresh_dir(&internal_path,0o700,empty.iter())?;
        let score_build_path = internal_path.join("score_build");
        util::recursive_refresh_dir(&score_build_path,0o700,empty.iter())?;


        let asgn_path = self.base_path.join(asgn);

        for member in self.members.iter() {
            let asgn_sub_path = asgn_path.join(member);

            let facl_list = self.grader_facl(Some(&member))?;

            util::refresh_dir(asgn_sub_path.clone(),0o700,facl_list.iter())?;

            let extension_path = asgn_sub_path.join(".extension");
            util::refresh_file(extension_path,0o700,"value = 0".to_string())?;

        }

        Ok(())
    }




    pub fn refresh(&self) -> Result<(),FailLog>
    {

        self.refresh_root()?;

        for asgn in self.manifest.iter() {
            self.refresh_assignment(asgn)?;
        }

        Ok(())

    }


    pub fn get_slot<'a>(&'a self, asgn: &'a AsgnSpec, user: &OsString) -> SubmissionSlot<'a> {
        let sub_dir = self.base_path.join(&asgn.name).join(user);

        SubmissionSlot {
            context:   &self,
            asgn_spec: asgn,
            base_path: sub_dir,
        }
    }

    fn offset_date(date : Option<&DateTime<Local>>, offset : i64 ) -> Result<Option<DateTime<Local>>,FailLog>
    {
        if let Some(date) = date.as_ref() {
            //println!("??? {}",date);
            //println!("!!! {}",date.checked_add_days(Days::new(offset as u64)).unwrap());
            let offset_date = if offset >= 0 {
                date.naive_local()
                    .checked_add_days(Days::new(offset as u64))
                    .ok_or(FailInfo::IOFail(format!("Extended date out of valid range.")))?
            } else {
                date.naive_local()
                    .checked_sub_days(Days::new(-offset as u64))
                    .ok_or(FailInfo::IOFail(format!("Extended date out of valid range.")))?
            };
            let offset_date = Local.from_local_datetime(&offset_date).single()
                .ok_or(FailInfo::IOFail(format!("Extended date out of valid range.")))?;
            Ok(Some(offset_date))
        } else {
            Ok(None)
        }
    }

    pub fn assignment_summary_row(&self, asgn: &AsgnSpec) -> Vec<String> {

        let active  = if asgn.active  {"YES"} else {"NO"};
        let visible = if asgn.visible {"YES"} else {"NO"};

        let status = if !asgn.active {
            "DISABLED"
        } else if asgn.before_open() {
            "BEFORE OPEN"
        } else if asgn.after_close() {
            "AFTER CLOSE"
        } else {
            "ENABLED"
        };

        let due_date  = asgn.due_date;
        let time_string = if let Some(time) = due_date {
            let time_string = time.time().to_string();
            if time_string == "23:59:59" {
                String::from("")
            } else {
                String::from(" ") + &time_string
            }
        } else {
            String::from("")
        };
        let naive_due_date  = due_date.map(|date| {
                date.date_naive().to_string() + &time_string
            }).unwrap_or(String::from("NONE"));


        let file_list : String = asgn.file_list.iter()
            .enumerate()
            .fold(String::new(),|acc,(idx,text)| {
                if idx == 0 {
                    text.to_string_lossy().to_string()
                } else {
                    format!("{}  {}",acc,text.to_string_lossy().to_string())
                }
            });

        vec![
            asgn.name.clone(),
            status.to_string(),
            active.to_string(),
            visible.to_string(),
            naive_due_date,
            file_list,
        ]
    }

    pub fn submission_summary_row(&self, asgn: &AsgnSpec, user: &OsString) -> Vec<String> {

        let slot = self.get_slot(asgn,user);
        let status = slot.status().unwrap();
        let lateness = status.versus(asgn.due_date.as_ref());

        let extension = status.extension_days;
        let grace     = status.grace_days;

        vec![
            asgn.name.clone(),
            user.to_string_lossy().to_string(),
            lateness.to_string(),
            extension.to_string(),
            grace.to_string()
        ]
    }

    pub fn normal_summary_row(&self, asgn: &AsgnSpec, user: &OsString) -> Result<Vec<String>,FailLog> {
        let sub_dir = self.base_path.join(&asgn.name).join(user);

        let slot = SubmissionSlot {
            context:   &self,
            asgn_spec: asgn,
            base_path: sub_dir,
        };

        let status = slot.status().unwrap();
        let due_date  = asgn.due_date;
        let extension = status.extension_days;
        let grace     = status.grace_days;
        let bump = extension+grace;

        let ext_due_date = Self::offset_date(asgn.due_date.as_ref(),bump)?;
        //if let Some(time) = ext_due_date {
        //    println!(">>> {} + {} -> {}",asgn.due_date.as_ref().unwrap(),bump,time);
        //}
        let lateness = status.versus(ext_due_date.as_ref());

        let bump : String = if bump != 0 {format!(" {:+}",bump)} else { String::new() };
        let active = if !asgn.active {
            "DISABLED"
        } else if asgn.before_open() {
            "BEFORE OPEN"
        } else if asgn.after_close() {
            "AFTER CLOSE"
        } else {
            "ENABLED"
        };

        let file_list : String = asgn.file_list.iter()
            .enumerate()
            .fold(String::new(),|acc,(idx,text)| {
                if idx == 0 {
                    text.to_string_lossy().to_string()
                } else {
                    format!("{}  {}",acc,text.to_string_lossy().to_string())
                }
            });

        let time_string = if let Some(time) = due_date {
            let time_string = time.time().to_string();
            if time_string == "23:59:59" {
                String::from("")
            } else {
                String::from(" ") + &time_string
            }
        } else {
            String::from("")
        };


        let naive_due_date  = due_date.map(|date| {
                date.date_naive().to_string() + &bump + &time_string
            }).unwrap_or(String::from("NONE"));

        Ok(vec![
            asgn.name.clone(),
            active.to_string(),
            naive_due_date,
            lateness.to_string(),
            file_list.to_string()
        ])
    }


    pub fn list_subs(&self, asgn: Option<&OsString>, user: Option<&OsString>)
     -> Result<(),FailLog>
    {
        let header : Vec<String> = ["ASSIGNMENT","USER", "SUBMISSION STATUS", "EXTENSION", "GRACE"]
            .iter().map(|s| s.to_string()).collect();

        let mut table = Table::new(header.len());
        table.add_row(header)?;


        let asgn_list : Vec<&OsString> = if let Some(asgn_name) = asgn {
            if ! self.manifest.contains(asgn_name) {
                return Err(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())
            }
            vec![asgn_name]
        } else {
            self.manifest.iter().collect()
        };

        let user_list : Vec<&OsString> = if let Some(user_name) = user {
            if ! self.members.contains(user_name) {
                return Err(FailInfo::InvalidUser(user_name.clone()).into_log())
            }
            vec![user_name]
        } else {
            self.members.iter().collect()
        };

        let body : Vec<Vec<String>> = asgn_list.iter()
            .filter_map(|name| self.catalog.get(name.clone()) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .cartesian_product(user_list.iter())
            .map(|(asgn,user)| {
                self.submission_summary_row(asgn, user)
            }).collect();

        for row in body.into_iter() {
            table.add_row(row)?;
        }

        print!("{}",table.as_table());

        Ok(())
    }

    pub fn list_asgns(&self)
     -> Result<(),FailLog>
    {
        let header : Vec<String> = ["NAME", "STATUS", "ACTIVE", "VISIBLE", "DUE", "FILES"]
            .iter().map(|s| s.to_string()).collect();

        let mut table = Table::new(header.len());
        table.add_row(header)?;


        let body : Vec<Vec<String>> = self.manifest.iter()
            .filter_map(|name| self.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .map(|asgn| {
                self.assignment_summary_row(asgn)
            }).collect();

        for row in body.into_iter() {
            table.add_row(row)?;
        }

        print!("{}",table.as_table());

        Ok(())
    }

    pub fn summary(&self) -> Result<(),FailLog>
    {

        let header : Vec<String> = ["ASSIGNMENT", "STATUS", "DUE DATE", "SUBMISSION STATUS", "FILES"]
                .iter().map(|s| s.to_string()).collect();

        let mut table = Table::new(header.len());
        table.add_row(header)?;

        let body : Vec<Vec<String>> = self.manifest.iter()
            .filter_map(|name| self.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .filter(|asgn| asgn.visible)
            .map(|asgn| self.normal_summary_row(asgn,&self.user))
            .filter_map(|row| row.ok())
            .collect();

        for row in body.into_iter() {
            table.add_row(row)?;
        }

        if self.grace_total.unwrap_or(0) != 0 {
            print!("TOTAL GRACE: {}    ",self.grace_total.unwrap());
            print!("GRACE LIMIT: {}    ",self.grace_limit
                .map(|limit| limit.to_string())
                .unwrap_or("NONE".to_string())
            );
            print!("GRACE SPENT: {}\n",self.grace_spent());
        }

        print!("{}",table.as_table());

        Ok(())
    }


    pub fn grace_spent(&self) -> i64 {
        self.manifest.iter()
            .filter_map(|name| self.catalog.get(&name.clone()) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .map(|asgn|{
                let sub_dir = self.base_path.join(&asgn.name).join(&self.user);

                let slot = SubmissionSlot {
                    context:   &self,
                    asgn_spec: asgn,
                    base_path: sub_dir,
                };

                let status = slot.status().unwrap();

                status.grace_days
            })
            .fold(0,|acc,val| acc + val)

    }


}



pub fn init(base_path : &Path) -> Result<(),FailLog> {
    if base_path.exists() {
        util::refresh_dir(&base_path,0o755,Vec::new().iter())?;
        let info_path = base_path.join(".info");
        util::refresh_dir(&info_path,0o755,Vec::new().iter())?;
        let toml_path = info_path.join("course.toml");
        util::refresh_file(&toml_path,0o755,toml::to_string(&CourseToml::default()).unwrap())?;
        let mut context = Context::deduce(OsString::from(base_path))?;
        InstructorAct::Refresh{}.execute(&mut context)
    } else {
        Err(FailInfo::Custom(
            "Course directory path does not exist.".to_string(),
            "Ensure that the provided path corresponds to an existing directory.".to_string(),
        ).into_log())
    }
}


