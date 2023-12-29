
use std::{
    collections::HashMap,
    env::current_dir,
    fs,
    os::unix::fs::{PermissionsExt, MetadataExt},
    path::{Path, PathBuf}, iter
};

use chrono::{TimeZone, DateTime, Local, Days, NaiveTime};
use users::{ get_user_by_uid, get_current_uid};
use serde_derive::{ Serialize, Deserialize};
use itertools::Itertools;

use crate::{
    fail_info::{FailInfo, FailLog},
    asgn_spec::{AsgnSpec, AsgnSpecToml, SubmissionSlot},
    util,
    table::Table,
    act::instructor::InstructorAct,
};

const DEFAULT_DUE_TIME: NaiveTime = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

#[derive(Default, Serialize, Deserialize)]
pub struct CourseToml {
    manifest:    Vec<String>,
    graders:     Vec<String>,
    students:    Vec<String>,
    grace_total: Option<i64>,
    grace_limit: Option<i64>,
}

impl From<&Context> for CourseToml {
    fn from(ctx: &Context) -> Self {
        Self {
            manifest: ctx.manifest.clone(),
            graders:  ctx.graders.clone(),
            students: ctx.students.clone(),
            grace_total: ctx.grace_total,
            grace_limit: ctx.grace_limit,
        }
    }
}

#[derive(PartialEq)]
pub enum Role {
    Instructor,
    Grader,
    Student,
    Other,
}

pub struct Context {
    // Determined through input
    pub instructor: String,
    pub base_path: PathBuf,
    pub exe_path: PathBuf,

    // Determined through system calls
    pub uid: u32,
    pub user: String,
    pub time: DateTime<Local>,
    pub cwd: PathBuf,

    // Determined by reading the context file
    pub manifest: Vec<String>,
    pub graders: Vec<String>,
    pub students: Vec<String>,
    pub members: Vec<String>,
    pub grace_total: Option<i64>,
    pub grace_limit: Option<i64>,

    // Determined by the context file + system calls
    pub role: Role,

    // Determined by trying to parse the spec of every
    // assignment in the manifest
    pub catalog: HashMap<String, Result<AsgnSpec, FailLog>>,
}

impl Context {
    fn load(base_path: &PathBuf) -> Result<CourseToml, FailLog> {
        let course_file_path = base_path.join(".info").join("course.toml");

        let course_file_text = fs::read_to_string(course_file_path).map_err(|err|
            FailInfo::NoSpec("course".into(), err.to_string()).into_log()
        )?;

        toml::from_str(&course_file_text).map_err(|err|
            FailInfo::BadSpec("course".into(), err.to_string()).into_log()
        )
    }

    pub fn sync(&self) -> Result<(), FailLog> {
        use FailInfo::*;
        let course_file_path = self.base_path.join(".info").join("course.toml");

        let course_toml = CourseToml::from(self);

        let toml_text = toml::to_string(&course_toml).map_err(|err|
            IOFail(format!("Could not serialize course spec: {}", err)).into_log()
        )?;

        util::write_file(course_file_path, toml_text)
    }

    pub fn populate_catalog(&mut self) {
        for asgn_name in self.manifest.iter() {
            let spec_path = self.base_path
                .join(asgn_name);
            self.catalog.insert(asgn_name.clone(), AsgnSpec::load(spec_path));
        }
    }

    pub fn deduce(base_path: impl AsRef<Path>) -> Result<Self, FailLog> {
        let base_path = base_path.as_ref().canonicalize().unwrap();

        let uid = get_current_uid();
        let user = get_user_by_uid(uid)
            .ok_or(FailInfo::InvalidUID())?
            .name().to_str().unwrap()
            .to_owned();

        let cwd = current_dir().map_err(|_| FailInfo::InvalidCWD())?;

        let exe_path = std::fs::read_link("/proc/self/exe")
            .map_err(|err| FailInfo::IOFail(err.to_string()).into_log())?;

        if !base_path.is_dir() {
            return Err(FailInfo::NoBaseDir(base_path).into());
        }

        let instructor_uid = std::fs::metadata(&base_path)
            .map_err(|err| FailInfo::IOFail(err.to_string()))?
            .uid();
        let instructor: String = get_user_by_uid(instructor_uid)
                .ok_or(FailInfo::InvalidUID())?
                .name().to_str().unwrap().to_owned();

        let time = Local::now();

        let spec = Self::load(&base_path)?;
        let manifest: Vec<_> = spec.manifest.clone();
        let graders:  Vec<_> = spec.graders .clone();
        let students: Vec<_> = spec.students.clone();
        let grace_total = spec.grace_total;
        let grace_limit = spec.grace_limit;

        let role =
            if spec.students.contains(&user) { Role::Student }
            else if spec.graders.contains(&user) { Role::Grader }
            else if user == instructor { Role::Instructor }
            else { Role::Other };

        let members = iter::once(instructor.clone())
            .chain(spec.graders.iter().cloned())
            .chain(spec.students.iter().cloned())
            .collect();

        let mut context = Self {
            instructor,
            base_path,
            exe_path,
            uid,
            user,
            time,
            cwd,
            manifest: spec.manifest,
            graders: spec.graders,
            students: spec.students,
            members,
            grace_total,
            grace_limit,
            role,
            catalog: Default::default(),
        };

        context.populate_catalog();
        Ok(context)
    }


    pub fn print_manifest(&self) {
        for name in self.manifest.iter() {
            println!("{name}");
        }
    }

    pub fn collect_failures(&self) -> FailLog {
        self.manifest.iter()
            .flat_map(|name| self.catalog[name].as_ref().err())
            .cloned()
            .flatten()
            .collect::<FailLog>()
    }

    pub fn get_spec<'a>(manifest: &'a Vec<AsgnSpec>, name: &str) -> Option<&'a AsgnSpec> {
        manifest.iter().find(|entry|
            name == &entry.name
        )
    }

    pub fn announce(&self) {
        println!("The time is: {}", self.time);
        println!("The user is: {}", self.user);
        println!("Called from directory {}", self.cwd.display());
    }

    fn make_dir_public<P: AsRef<Path>, L : AsRef<str>>(path: P, label: L) -> Result<(), FailLog> {
        let label = label.as_ref();
        let mut perm = fs::metadata(path.as_ref())
            .map_err(|err| FailInfo::IOFail(format!("Failed chmoding {label}: {err}")).into_log())?
            .permissions();

        perm.set_mode(0o755);
        Ok(())
    }

    pub fn grader_facl(&self, student: Option<&str>) -> Result<Vec<util::FaclEntry>, FailLog> {
        let mut facl_list: Vec<util::FaclEntry> = Vec::new();

        facl_list.push(util::FaclEntry {
            user: self.instructor.clone(),
            read: true,
            write: true,
            exe: true,
        });

        if let Some(student) = student {
            facl_list.push(util::FaclEntry {
                user: student.to_owned(),
                read: true,
                write: true,
                exe: true,
            });
        }

        let graders_exclusive = self.graders.iter()
            .filter(|&grader| Some(grader.as_str()) != student && grader != &self.instructor)
            .map(|grader| util::FaclEntry {
                user: grader.to_owned(),
                read: true,
                write: false,
                exe: true,
            });

        facl_list.extend(graders_exclusive);

        Ok(facl_list)
    }

    fn refresh_root(&self) -> Result<(), FailLog> {
        util::refresh_dir(&self.base_path, 0o755, iter::empty())?;

        let course_info_path = self.base_path.join(".info");
        let course_file_path = course_info_path.join("course.toml");
        let course_text = toml::to_string(&CourseToml::default()).unwrap();

        util::refresh_dir(&course_info_path, 0o755, iter::empty())?;
        util::refresh_file(&course_file_path, 0o644, course_text)?;

        let dirs = [
            ("public",  0o755, Vec::new()),
            ("private", 0o700, self.grader_facl(None)?),
        ];

        for (name, flags, facl) in dirs.iter() {
            let path = course_info_path.join(name);
            util::recursive_refresh_dir(&path, *flags, facl.iter())?;
        }

        Ok(())
    }


    pub fn refresh_assignment(&self, asgn: &str) -> Result<(), FailLog> {
        let asgn_path = self.base_path.join(asgn);
        util::refresh_dir(&asgn_path, 0o755, iter::empty())?;

        let asgn_spec_path = asgn_path.join(".info");
        util::refresh_dir(&asgn_spec_path, 0o755, iter::empty())?;

        let asgn_info_path = asgn_spec_path.join("info.toml");
        let asgn_text = toml::to_string(&AsgnSpecToml::default_with_name(asgn.to_owned())).unwrap();
        util::refresh_file(&asgn_info_path, 0o644, asgn_text)?;

        let asgn_make_path = asgn_spec_path.join("Makefile");
        util::refresh_file(&asgn_make_path, 0o644, String::new())?;

        let asgn_make_path = asgn_spec_path.join("score.toml");
        util::refresh_file(&asgn_make_path, 0o644, String::new())?;

        let dirs = [
            ("public", 0o755, Vec::new()),
            ("private", 0o700, self.grader_facl(None)?),
        ];

        for (name, flags, facl) in dirs.into_iter() {
            let path = asgn_spec_path.join(name);
            util::recursive_refresh_dir(&path, flags, facl.iter())?;
        }

        let internal_path = asgn_spec_path.join(".internal");
        let score_build_path = internal_path.join("score_build");
        util::recursive_refresh_dir(&internal_path, 0o700, iter::empty())?;
        util::recursive_refresh_dir(&score_build_path, 0o700, iter::empty())?;

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

    pub fn refresh(&self) -> Result<(), FailLog> {
        self.refresh_root()?;

        for asgn in self.manifest.iter() {
            self.refresh_assignment(asgn)?;
        }

        Ok(())
    }


    pub fn get_slot<'a>(&'a self, asgn: &'a AsgnSpec, user: &str) -> SubmissionSlot<'a> {
        SubmissionSlot {
            context: &self,
            asgn_spec: asgn,
            base_path: self.base_path.join(&asgn.name).join(user),
        }
    }

    fn offset_date(date: Option<&DateTime<Local>>, offset: i64) -> Result<Option<DateTime<Local>>, FailLog> {
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
        let active  = if asgn.active  { "YES" } else { "NO" };
        let visible = if asgn.visible { "YES" } else { "NO" };

        let status =
            if !asgn.active { "DISABLED" }
            else if asgn.before_open() { "BEFORE OPEN" }
            else if asgn.after_close() { "AFTER CLOSE" }
            else { "ENABLED" };

        let naive_due_date = match asgn.due_date {
            None => "NONE".to_owned(),
            Some(due) => {
                let date = due.date_naive();
                match due.time() {
                    DEFAULT_DUE_TIME => date.to_string(),
                    time => format!("{date} {time}"),
                }
            }
        };

        vec![
            asgn.name.clone(),
            status.to_string(),
            active.to_string(),
            visible.to_string(),
            naive_due_date,
            asgn.file_list.iter().join("  "),
        ]
    }

    pub fn submission_summary_row(&self, asgn: &AsgnSpec, user: &str) -> Vec<String> {
        let status = self.get_slot(asgn, user).status().unwrap();
        let lateness = status.versus(asgn.due_date.as_ref());

        let extension = status.extension_days;
        let grace = status.grace_days;

        vec![
            asgn.name.clone(),
            user.to_owned(),
            lateness,
            extension.to_string(),
            grace.to_string(),
        ]
    }

    pub fn normal_summary_row(&self, asgn: &AsgnSpec, user: &str) -> Result<Vec<String>, FailLog> {
        let sub_dir = self.base_path.join(&asgn.name).join(user);

        let slot = SubmissionSlot {
            context:   &self,
            asgn_spec: asgn,
            base_path: sub_dir,
        };

        let status = slot.status().unwrap();
        let due_date = asgn.due_date;
        let extension = status.extension_days;
        let grace = status.grace_days;
        let bump = extension+grace;

        let ext_due_date = Self::offset_date(asgn.due_date.as_ref(), bump)?;
        let lateness = status.versus(ext_due_date.as_ref());

        let active =
            if !asgn.active { "DISABLED" }
            else if asgn.before_open() { "BEFORE OPEN" }
            else if asgn.after_close() { "AFTER CLOSE" }
            else { "ENABLED" };


        let naive_due_date = match due_date {
            None => "NONE".to_owned(),
            Some(due) => {
                let date = due.date_naive();
                match due.time() {
                    DEFAULT_DUE_TIME => due.to_string(),
                    time => format!("{date} {time}"),
                }
            }
        };

        Ok(vec![
            asgn.name.clone(),
            active.to_owned(),
            naive_due_date,
            lateness,
            asgn.file_list.iter().join("  ")
        ])
    }


    pub fn list_subs(&self, asgn: Option<&str>, user: Option<&str>) -> Result<(), FailLog> {
        let header: Vec<String> = ["ASSIGNMENT","USER", "SUBMISSION STATUS", "EXTENSION", "GRACE"]
            .iter().map(|s| s.to_string()).collect();

        let mut table = Table::new(header.len());
        table.add_row(header)?;

        let asgn_list: Vec<_> = match asgn {
            Some(asgn_name) => {
                if !self.manifest.iter().any(|asgn| asgn == asgn_name) {
                    return Err(FailInfo::InvalidAsgn(asgn_name.to_owned()).into_log())
                }
                vec![asgn_name]
            }
            None => self.manifest.iter().map(String::as_str).collect(),
        };

        let user_list: Vec<_> = match user {
            Some(user_name) => {
                if !self.members.iter().any(|member| member == user_name) {
                    return Err(FailInfo::InvalidUser(user_name.to_owned()).into_log())
                }
                vec![user_name]
            }
            None => self.members.iter().map(String::as_str).collect(),
        };

        let body: Vec<Vec<String>> = asgn_list.iter()
            .filter_map(|&name| self.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok())
            .cartesian_product(user_list.iter())
            .map(|(asgn, user)| self.submission_summary_row(asgn, user))
            .collect();

        for row in body.into_iter() {
            table.add_row(row)?;
        }

        print!("{table}");

        Ok(())
    }

    pub fn list_asgns(&self) -> Result<(), FailLog> {
        let header: Vec<String> = ["NAME", "STATUS", "ACTIVE", "VISIBLE", "DUE", "FILES"].into_iter()
            .map(str::to_owned)
            .collect();

        let mut table = Table::new(header.len());
        table.add_row(header)?;


        let body: Vec<Vec<String>> = self.manifest.iter()
            .filter_map(|name| self.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .map(|asgn| self.assignment_summary_row(asgn))
            .collect();

        for row in body.into_iter() {
            table.add_row(row)?;
        }

        print!("{table}");

        Ok(())
    }

    pub fn summary(&self) -> Result<(), FailLog> {
        let header : Vec<String> = ["ASSIGNMENT", "STATUS", "DUE DATE", "SUBMISSION STATUS", "FILES"]
            .iter().map(|s| s.to_string()).collect();

        let mut table = Table::new(header.len());
        table.add_row(header)?;

        let body : Vec<Vec<String>> = self.manifest.iter()
            .filter_map(|name| self.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .filter(|asgn| asgn.visible)
            .map(|asgn| self.normal_summary_row(asgn, &self.user))
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

        print!("{table}");

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
            .sum()
    }
}



pub fn init(base_path: impl AsRef<Path>) -> Result<(), FailLog> {
    let base_path = base_path.as_ref();

    if !base_path.exists() {
        return Err(FailInfo::Custom(
            "Course directory path does not exist.".to_string(),
            "Ensure that the provided path corresponds to an existing directory.".to_string(),
        ).into_log())
    }

    util::refresh_dir(&base_path,0o755,iter::empty())?;

    let info_path = base_path.join(".info");
    util::refresh_dir(&info_path,0o755,iter::empty())?;

    let toml_path = info_path.join("course.toml");
    util::refresh_file(&toml_path,0o755,toml::to_string(&CourseToml::default()).unwrap())?;

    let mut context = Context::deduce(&base_path)?;
    InstructorAct::Refresh{}.execute(&mut context)
}
