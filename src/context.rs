
use std::{
    collections::HashMap,
    env::current_dir,
    fs,
    os::unix::fs::{PermissionsExt, MetadataExt},
    path::{Path, PathBuf}, iter
};

use chrono::{TimeZone, DateTime, Local, Days};
use users::{ get_user_by_uid, get_current_uid};
use serde_derive::{ Serialize, Deserialize};
use itertools::Itertools;

use crate::{
    error::{Error, ErrorLog},
    asgn_spec::{AsgnSpec, AsgnSpecToml, SubmissionSlot},
    util,
    table::Table,
    act::instructor::InstructorAct,
};

#[derive(Default, Serialize, Deserialize)]
pub struct CourseToml {
    manifest: Vec<String>,
    graders: Vec<String>,
    students: Vec<String>,
    grace_total: Option<i64>,
    grace_limit: Option<i64>,
}

impl From<&Context> for CourseToml {
    fn from(ctx: &Context) -> Self {
        Self {
            manifest: ctx.manifest.clone(),
            graders: ctx.graders.clone(),
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
    pub username: String,
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

    // Determined by trying to parse the spec of every assignment in the manifest
    pub catalog: HashMap<String, Result<AsgnSpec, Error>>,
}

#[allow(dead_code)]
impl Context {
    fn load(base_path: &Path) -> Result<CourseToml, Error> {
        let course_file_path = base_path.join(".info").join("course.toml");

        let course_file_text = fs::read_to_string(&course_file_path).map_err(|err|
            Error::SpecIo(course_file_path.clone(), err.kind())
        )?;

        toml::from_str(&course_file_text).map_err(|err|
            Error::InvalidToml(course_file_path, err)
        )
    }

    pub fn sync(&self) -> Result<(), Error> {
        let course_file_path = self.base_path.join(".info").join("course.toml");

        let course_toml = CourseToml::from(self);

        let toml_text = toml::to_string(&course_toml).map_err(|err|
            Error::TomlSer("CourseToml", err)
        )?;

        fs::write(&course_file_path, toml_text).map_err(|err|
            Error::Io("Failed writing course file", course_file_path, err.kind())
        )
    }

    pub fn populate_catalog(&mut self) {
        for asgn_name in &self.manifest {
            let spec_path = self.base_path.join(asgn_name);
            self.catalog.insert(asgn_name.clone(), AsgnSpec::load(spec_path));
        }
    }

    pub fn catalog_get<'a>(&'a self, asgn_name: &str) -> Result<&'a AsgnSpec, Error> {
        self.catalog.get(asgn_name)
            .ok_or(Error::InvalidAsgn { name: asgn_name.to_owned() })?
            .as_ref()
            .map_err(Clone::clone)
    }

    pub fn catalog_get_mut<'a>(&'a mut self, asgn_name: &str) -> Result<&'a mut AsgnSpec, Error> {
        self.catalog.get_mut(asgn_name)
            .ok_or(Error::InvalidAsgn { name: asgn_name.to_owned() })?
            .as_mut()
            .map_err(|err| err.clone())
    }

    pub fn deduce(base_path: impl AsRef<Path>) -> Result<Self, Error> {
        const PROC_SELF_EXE: &str = "/proc/self/exe";
        let base_path = base_path.as_ref().canonicalize().unwrap();

        let uid = get_current_uid();
        let username = get_user_by_uid(uid)
            .ok_or(Error::InvalidUID(uid))?
            .name().to_str().unwrap()
            .to_owned();

        let cwd = current_dir().map_err(|_| Error::InvalidCWD)?;

        let exe_path = std::fs::read_link(PROC_SELF_EXE).map_err(|err|
            Error::Io("Failed to read process' EXE path", PathBuf::from(PROC_SELF_EXE), err.kind())
        )?;

        if !base_path.is_dir() {
            return Err(Error::NoBaseDir(base_path));
        }

        let instructor_uid = std::fs::metadata(&base_path)
            .map_err(|err| Error::Io("Failed to stat file", base_path.clone(), err.kind()))?
            .uid();

        let instructor = get_user_by_uid(instructor_uid)
                .ok_or(Error::InvalidUID(uid))?
                .name().to_str().unwrap().to_owned();

        let time = Local::now();

        let toml = Self::load(&base_path)?;
        let grace_total = toml.grace_total;
        let grace_limit = toml.grace_limit;

        let role =
            if toml.students.contains(&username) { Role::Student }
            else if toml.graders.contains(&username) { Role::Grader }
            else if username == instructor { Role::Instructor }
            else { Role::Other };

        let members = iter::once(instructor.clone())
            .chain(toml.graders.iter().cloned())
            .chain(toml.students.iter().cloned())
            .collect();

        let mut context = Self {
            instructor,
            base_path,
            exe_path,
            uid,
            username,
            time,
            cwd,
            manifest: toml.manifest,
            graders: toml.graders,
            students: toml.students,
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
        for name in &self.manifest {
            println!("{name}");
        }
    }

    pub fn all_catalog_errors(&self) -> ErrorLog {
        self.manifest.iter()
            .flat_map(|name| self.catalog[name].as_ref().err())
            .cloned()
            .collect::<ErrorLog>()
    }

    pub fn announce(&self) {
        println!("The time is: {}", self.time);
        println!("The username is: {}", self.username);
        println!("Called from directory {}", self.cwd.display());
    }

    fn make_dir_public<P: AsRef<Path>, L: AsRef<str>>(path: impl AsRef<Path>) -> Result<(), Error> {
        let mut perm = fs::metadata(path.as_ref())
            .map_err(|err| Error::Io("Failed to stat file", path.as_ref().to_owned(), err.kind()))?
            .permissions();

        perm.set_mode(0o755);
        Ok(())
    }

    pub fn grader_facl(&self, student: Option<&str>) -> Result<Vec<util::FaclEntry>, Error> {
        let mut facl_list: Vec<util::FaclEntry> = Vec::new();

        facl_list.push(util::FaclEntry {
            username: self.instructor.clone(),
            read: true,
            write: true,
            exe: true,
        });

        if let Some(student) = student {
            facl_list.push(util::FaclEntry {
                username: student.to_owned(),
                read: true,
                write: true,
                exe: true,
            });
        }

        let graders_exclusive = self.graders.iter()
            .filter(|&grader| Some(grader.as_str()) != student && grader != &self.instructor)
            .map(|grader| util::FaclEntry {
                username: grader.to_owned(),
                read: true,
                write: false,
                exe: true,
            });

        facl_list.extend(graders_exclusive);

        Ok(facl_list)
    }

    fn refresh_root(&self) -> Result<(), Error> {
        util::refresh_dir(&self.base_path, 0o755, iter::empty())?;

        let course_info_path = self.base_path.join(".info");
        let course_file_path = course_info_path.join("course.toml");
        let course_text = toml::to_string(&CourseToml::default()).unwrap();

        util::refresh_dir(&course_info_path, 0o755, iter::empty())?;
        util::refresh_file(course_file_path, 0o644, &course_text)?;

        let dirs = [
            ("public",  0o755, Vec::new()),
            ("private", 0o700, self.grader_facl(None)?),
        ];

        for (name, flags, facl) in dirs {
            let path = course_info_path.join(name);
            util::recursive_refresh_dir(&path, flags, facl.iter())?;
        }

        Ok(())
    }

    pub fn refresh_assignment(&self, asgn_name: &str) -> Result<(), Error> {
        let asgn_path = self.base_path.join(asgn_name);
        util::refresh_dir(&asgn_path, 0o755, iter::empty())?;

        let asgn_spec_path = asgn_path.join(".info");
        util::refresh_dir(&asgn_spec_path, 0o755, iter::empty())?;

        let asgn_info_path = asgn_spec_path.join("info.toml");
        let asgn_text = toml::to_string(&AsgnSpecToml::default_with_name(asgn_name.to_owned())).unwrap();
        util::refresh_file(asgn_info_path, 0o644, &asgn_text)?;

        let asgn_make_path = asgn_spec_path.join("Makefile");
        util::refresh_file(asgn_make_path, 0o644, "")?;

        let asgn_make_path = asgn_spec_path.join("score.toml");
        util::refresh_file(asgn_make_path, 0o644, "")?;

        let dirs = [
            ("public", 0o755, Vec::new()),
            ("private", 0o700, self.grader_facl(None)?),
        ];

        for (name, flags, facl) in dirs {
            let path = asgn_spec_path.join(name);
            util::recursive_refresh_dir(&path, flags, facl.iter())?;
        }

        let internal_path = asgn_spec_path.join(".internal");
        let score_build_path = internal_path.join("score_build");
        util::recursive_refresh_dir(internal_path, 0o700, iter::empty())?;
        util::recursive_refresh_dir(score_build_path, 0o700, iter::empty())?;

        let asgn_path = self.base_path.join(asgn_name);

        for member in &self.members {
            let asgn_sub_path = asgn_path.join(member);
            let facl_list = self.grader_facl(Some(member))?;

            util::refresh_dir(asgn_sub_path.clone(), 0o700, facl_list.iter())?;

            let extension_path = asgn_sub_path.join(".extension");
            util::refresh_file(extension_path, 0o700, "value = 0")?;
        }

        Ok(())
    }

    pub fn refresh(&self) -> Result<(), Error> {
        self.refresh_root()?;

        self.manifest.iter().try_for_each(|asgn|
            self.refresh_assignment(asgn)
        )
    }

    pub fn get_slot<'a>(&'a self, asgn: &'a AsgnSpec, username: &str) -> SubmissionSlot<'a> {
        SubmissionSlot {
            context: self,
            asgn_spec: asgn,
            base_path: self.base_path.join(&asgn.name).join(username),
        }
    }

    fn offset_date(date: Option<&DateTime<Local>>, offset: i64) -> Result<Option<DateTime<Local>>, Error> {
        if let Some(&date) = date {
            let offset_date = if offset >= 0 {
                date.naive_local()
                    .checked_add_days(Days::new(offset as u64))
                    .ok_or(Error::DateOutOfRange(date))?
            } else {
                date.naive_local()
                    .checked_sub_days(Days::new(-offset as u64))
                    .ok_or(Error::DateOutOfRange(date))?
            };
            let offset_date = Local.from_local_datetime(&offset_date).single()
                .ok_or(Error::DateOutOfRange(date))?;

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

        let naive_due_date = asgn.due_date.map(|due| {
            let date = due.date_naive();
            match due.time() {
                util::DEFAULT_DUE_TIME => date.to_string(),
                time => format!("{date} {time}"),
            }
        });

        vec![
            asgn.name.clone(),
            status.to_string(),
            active.to_string(),
            visible.to_string(),
            Table::option_repr(naive_due_date),
            asgn.file_list.iter().map(|f| f.display()).join("  "),
        ]
    }

    pub fn submission_summary_row(&self, asgn: &AsgnSpec, username: &str) -> Vec<String> {
        let status = self.get_slot(asgn, username).status().unwrap();
        let lateness = status.versus(asgn.due_date.as_ref());

        let extension = status.extension_days;
        let grace = status.grace_days;

        vec![
            asgn.name.clone(),
            username.to_owned(),
            lateness,
            extension.to_string(),
            grace.to_string(),
        ]
    }

    pub fn normal_summary_row(&self, asgn: &AsgnSpec, username: &str) -> Result<Vec<String>, Error> {
        let sub_dir = self.base_path.join(&asgn.name).join(username);

        let slot = SubmissionSlot {
            context: self,
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

        let naive_due_date = due_date.map(|due| {
            let date = due.date_naive();
            match due.time() {
                util::DEFAULT_DUE_TIME => due.to_string(),
                time => format!("{date} {time}"),
            }
        });

        Ok(vec![
            asgn.name.clone(),
            active.to_owned(),
            Table::option_repr(naive_due_date),
            lateness,
            asgn.file_list.iter().map(|f| f.display()).join("  ")
        ])
    }

    pub fn list_subs(&self, asgn_name: Option<&str>, username: Option<&str>) -> Result<(), Error> {
        let header = ["ASSIGNMENT", "USER", "SUBMISSION STATUS", "EXTENSION", "GRACE"].map(str::to_owned);

        let mut table = Table::new(header);

        let asgn_names: Vec<_> = match asgn_name {
            Some(name) => {
                if !self.manifest.iter().any(|asgn| asgn == name) {
                    return Err(Error::InvalidAsgn { name: name.to_owned() })
                }
                vec![name]
            }
            None => self.manifest.iter().map(String::as_str).collect(),
        };

        let usernames: Vec<_> = match username {
            Some(username) => {
                if !self.members.iter().any(|member| member == username) {
                    return Err(Error::InvalidUser(username.to_owned()))
                }
                vec![username]
            }
            None => self.members.iter().map(String::as_str).collect(),
        };

        table.extend(asgn_names.iter()
            .filter_map(|&asgn_name| self.catalog.get(asgn_name) )
            .filter_map(|asgn| asgn.as_ref().ok())
            .cartesian_product(usernames.iter())
            .map(|(asgn, username)| self.submission_summary_row(asgn, username))
        )?;

        print!("{table}");

        Ok(())
    }

    pub fn list_asgns(&self) -> Result<(), Error> {
        let header = ["NAME", "STATUS", "ACTIVE", "VISIBLE", "DUE", "FILES"].map(str::to_owned);

        let mut table = Table::new(header);

        table.extend(self.manifest.iter()
            .filter_map(|name| self.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .map(|asgn| self.assignment_summary_row(asgn))
        )?;

        print!("{table}");

        Ok(())
    }

    pub fn summary(&self) -> Result<(), Error> {
        let mut table = Table::new(["ASSIGNMENT", "STATUS", "DUE DATE", "SUBMISSION STATUS", "FILES"].map(str::to_owned));

        table.extend(self.manifest.iter()
            .filter_map(|name| self.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .filter(|asgn| asgn.visible)
            .map(|asgn| self.normal_summary_row(asgn, &self.username))
            .filter_map(|row| row.ok())
        )?;

        if self.grace_total.unwrap_or_default() != 0 {
            print!("TOTAL GRACE: {}    ", self.grace_total.unwrap_or_default());
            print!("GRACE LIMIT: {}    ", Table::option_repr(self.grace_limit.as_ref()));
            println!("GRACE SPENT: {}", self.grace_spent());
        }

        print!("{table}");

        Ok(())
    }

    pub fn grace_spent(&self) -> i64 {
        self.manifest.iter()
            .filter_map(|name| self.catalog.get(&name.clone()) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .map(|asgn|{
                let sub_dir = self.base_path.join(&asgn.name).join(&self.username);

                let slot = SubmissionSlot {
                    context: self,
                    asgn_spec: asgn,
                    base_path: sub_dir,
                };

                let status = slot.status().unwrap();

                status.grace_days
            })
            .sum()
    }
}

pub fn init(base_path: impl AsRef<Path>) -> Result<(), ErrorLog> {
    let base_path = base_path.as_ref();

    if !base_path.exists() {
        return Err(Error::Custom(
            "Course directory path does not exist.".to_owned(),
            "Ensure that the provided path corresponds to an existing directory.".to_owned(),
        ).into());
    }

    util::refresh_dir(base_path, 0o755, iter::empty())?;

    let info_path = base_path.join(".info");
    util::refresh_dir(&info_path, 0o755, iter::empty())?;

    let toml_path = info_path.join("course.toml");
    util::refresh_file(toml_path, 0o755, &toml::to_string(&CourseToml::default()).unwrap())?;

    let mut context = Context::deduce(base_path)?;
    InstructorAct::Refresh{}.execute(&mut context)
}
