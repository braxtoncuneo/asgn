use std::{
    ffi::{OsString, OsStr},
    fs,
    path::{PathBuf, Path},
    process::Stdio,
    os::unix::fs::MetadataExt,
};

use itertools::Itertools;
use serde_derive::{Serialize, Deserialize};
use chrono::{DateTime, Local, TimeZone, Duration};
use users::get_user_by_uid;
use toml;

use crate::{
    fail_info::{FailInfo, FailLog},
    context::{Context, Role},
    util,
    table::Table,
};

use colored::Colorize;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rule {
    pub target: String,
    pub fail_okay: Option<bool>,
    pub wait_text: Option<String>,
    pub pass_text: Option<String>,
    pub fail_text: Option<String>,
    pub help_text: Option<String>,
    pub kind: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ruleset {
    pub on_grade: Option<bool>,
    pub on_submit: Option<bool>,
    pub fail_okay: Option<bool>,
    pub rules: Vec<Rule>,
}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatBlock {
    pub user: String,
    pub time: toml::value::Datetime,
    pub scores: toml::value::Table,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StatBlockSet {
    pub stat_block: Option<Vec<StatBlock>>
}



#[derive(Serialize,Deserialize)]
pub struct AsgnSpecToml {
    name: String,
    active: bool,
    visible: bool,
    due_date: Option<toml::value::Datetime>,
    open_date: Option<toml::value::Datetime>,
    close_date: Option<toml::value::Datetime>,
    file_list: Vec<String>,

    build: Option<Ruleset>,
    grade: Option<Ruleset>,
    check: Option<Ruleset>,
    score: Option<Ruleset>,
}

impl Default for AsgnSpecToml {
    fn default() -> Self {
        Self {
            name: "<put name here>".to_string(),
            active: false,
            visible: false,
            due_date: None,
            open_date: None,
            close_date: None,
            file_list: Vec::new(),
            build: None,
            check: None,
            grade: None,
            score: None,
        }
    }
}

impl AsgnSpecToml {
    pub fn default_with_name <S: AsRef<OsStr>> (name : S) -> Self {
        let mut result : Self = Default::default();
        result.name = name.as_ref().to_string_lossy().to_string();
        result.file_list.push(format!("{}.cpp",&result.name));
        result
    }
}

impl From<AsgnSpec> for AsgnSpecToml {
    fn from(spec: AsgnSpec) -> Self {
        AsgnSpecToml {
            name: spec.name,
            active: spec.active,
            visible: spec.visible,
            due_date: spec.due_date.map(util::date_from_chrono),
            open_date: spec.open_date.map(util::date_from_chrono),
            close_date: spec.close_date.map(util::date_from_chrono),
            file_list: spec.file_list.clone(),
            build: spec.build,
            check: spec.check,
            grade: spec.grade,
            score: spec.score,
        }
    }
}

impl From<&AsgnSpec> for AsgnSpecToml {
    fn from(spec: &AsgnSpec) -> Self {
        Self::from(spec.clone())
    }
}

#[derive(Clone)]
pub struct AsgnSpec {
    pub name: String,
    pub active: bool,
    pub visible: bool,
    pub due_date: Option<DateTime<Local>>,
    pub open_date: Option<DateTime<Local>>,
    pub close_date: Option<DateTime<Local>>,
    pub file_list: Vec<String>,
    pub build: Option<Ruleset>,
    pub grade: Option<Ruleset>,
    pub check: Option<Ruleset>,
    pub score: Option<Ruleset>,
    pub path: PathBuf,
}

impl AsgnSpec {
    pub fn from_toml(toml: AsgnSpecToml, path: PathBuf) -> Result<Self, FailLog> {
        let open_date = match toml.open_date {
            Some(date) => Some(util::date_into_chrono(date)?),
            None => None,
        };

        let close_date = match toml.close_date {
            Some(date) => Some(util::date_into_chrono(date)?),
            None => None,
        };

        let due_date = match toml.due_date {
            Some(date) => Some(util::date_into_chrono(date)?),
            None => None,
        };

        Ok(Self {
            name: toml.name,
            active: toml.active,
            visible: toml.visible,
            due_date,
            open_date,
            close_date,
            file_list: toml.file_list,
            build: toml.build,
            check: toml.check,
            grade: toml.grade,
            score: toml.score,
            path,
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self,FailLog> {
        let path = path.as_ref();
        let spec_path = path.join(".info");
        let info_path = spec_path.join("info.toml");

        // println!("{}",info_path.display());

        let info_text = fs::read_to_string(info_path).map_err(|err|
            FailInfo::NoSpec(
                path.file_name().map(|os| os.to_str().unwrap().to_owned())
                    .unwrap_or("assignment".to_string()),
                err.to_string()
            ).into_log()
        )?;

        let spec_toml: AsgnSpecToml = toml::from_str(&info_text).map_err(|err|
            FailInfo::BadSpec("assignment".to_string(), err.to_string()).into_log()
        )?;

        let spec = Self::from_toml(spec_toml, path.to_owned())?;

        if !path.ends_with(&spec.name) {
            return Err(FailInfo::BadSpec(
                "assignment".to_string(),
                String::from("Name field does not match assignment directory name.")
            ).into());
        }

        Ok(spec)
    }

    pub fn sync(&self) -> Result<(), FailLog>{
        use FailInfo::*;

        let spec_toml = AsgnSpecToml::from(self);

        let toml_text = toml::to_string(&spec_toml)
            .map_err(|err| IOFail(format!(
                "Could not serialize course spec : {}",
                err
            )).into_log())?;

        util::write_file(
            self.path.join(".info").join("info.toml"),
            toml_text
        )
    }


    pub fn before_open(&self) -> bool {
        self.open_date.map(|date| {
            Local::now().checked_add_days(chrono::naive::Days::new(1)).unwrap()
            .signed_duration_since(date) < chrono::Duration::zero()
        }).unwrap_or(false)
    }

    pub fn after_close(&self) -> bool {
        self.close_date.map(|date| {
            Local::now().signed_duration_since(date) > chrono::Duration::zero()
        }).unwrap_or(false)
    }

    pub fn details(&self, context: &Context) -> Result<String,FailLog> {
        let sub_dir = context.base_path.join(&self.name).join(&context.user);

        let slot = SubmissionSlot {
            context:   context,
            asgn_spec: self,
            base_path: sub_dir,
        };

        let status = slot.status().unwrap();

        let mut table = Table::new(2);
        table.add_row(vec!["PROPERTY".to_string(),"VALUE".to_string()])?;
        table.add_row(vec!["NAME".to_string(), self.name.clone()])?;
        table.add_row(vec!["FILES".to_string(), self.file_list.iter().join(" ")])?;
        table.add_row(vec![
            "OPEN DATE".to_string(),
            self.open_date.as_ref().map(|d| d.to_string()).unwrap_or("NONE".to_string()),
        ])?;
        table.add_row(vec![
            "CLOSE DATE".to_string(),
            self.close_date.map(|d| d.to_string()).unwrap_or("NONE".to_string()),
        ])?;
        table.add_row(vec![
            "DUE DATE".to_string(),
            self.due_date.map(|d| d.to_string()).unwrap_or("NONE".to_string()),
        ])?;
        table.add_row(vec!["EXTENSION".to_string(), status.extension_days.to_string()])?;
        table.add_row(vec!["GRACE".to_string(), status.grace_days.to_string()])?;

        Ok(table.as_table())
    }

    pub fn make_command(&self, target: &str, quiet: bool, context: &Context) -> std::process::Command {
        let path = self.path.join(".info").join("Makefile");
        let mut cmd  = std::process::Command::new("make");

        if quiet {
            cmd.arg("--quiet");
        }
        cmd.arg(format!(
            "COURSE_PUBLIC={}",
            context.base_path.join(".info").join("public").display()
        ));
        cmd.arg(format!(
            "COURSE_PRIVATE={}",
            context.base_path.join(".info").join("private").display()
        ));
        cmd.arg(format!(
            "PUBLIC={}",
            self.path.join(".info").join("public").display()
        ));
        cmd.arg(format!(
            "PRIVATE={}",
            self.path.join(".info").join("private").display()
        ));
        cmd.arg(format!("--file={}",path.display()));
        cmd.arg(target);
        cmd
    }


    pub fn run_rule(&self, context: &Context, rule: &Rule, path: &Path) -> Result<bool, ()> {
        let wait_text = rule.wait_text.as_ref()
            .unwrap_or(&format!("Executing '{}'.",&rule.target))
            .yellow().bold();
        println!("{wait_text}");

        let quiet = match context.role {
            Role::Instructor => false,
            Role::Grader     => false,
            Role::Student    => true,
            Role::Other      => true,
        };

        let mut cmd = self.make_command(rule.target.as_ref(), quiet, context);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let (status, _, _) = util::run_at(cmd,&path,false).map_err(drop)?;

        if status.success() {
            let pass_text = rule.pass_text.clone()
                .unwrap_or(format!("'{}' passed.",&rule.target));
            let pass_text = format!("! {}", pass_text)
                .green();
            println!("{pass_text}");
            let target = path.join(&rule.target);
            if target.exists() {
                let _ = util::refresh_file(target, 0o777, String::new());
            }

            Ok(true)
        } else {
            let fail_text = rule.fail_text.clone()
                .unwrap_or(format!("'{}' failed.", &rule.target));
            let fail_text = format!("! {}", fail_text).red();
            let help_text = rule.help_text.as_ref().map(|t|t.yellow().to_string());
            if ! rule.fail_okay.unwrap_or(false) {
                println!("{fail_text}");
                if let Some(help) = help_text {
                    println!("{}",format!("> {}",help).yellow());
                }
                Err(())
            } else {
                println!("{fail_text}");
                if let Some(help) = help_text {
                    println!("{}",format!("> {}",help).yellow());
                }
                Ok(false)
            }
        }
    }


    fn log_metric(scores : &mut toml::value::Table, target : &str, result : &str, kind : &str)
    {
        let score = match kind {
            "bool"  => result.parse::<bool>().map(|v| toml::Value::Boolean(v)).ok(),
            "int"   => result.parse::<i64>() .map(|v| toml::Value::Integer(v)).ok(),
            "float" => result.parse::<f64>() .map(|v| toml::Value::Float  (v)).ok(),
            _ => {
                println!("{}",format!("! Metric '{}' has invalid kind '{}'",target,kind).red());
                return;
            },
        };
        if let Some(score) = score {
            scores.insert(target.to_string(), score);
            println!("{}",format!("Metric '{}' had value '{}'", target, result).yellow().bold());
        } else {
            println!("{}",format!(
                "Metric '{}' had result '{}' which could not be parsed into kind '{}'",
                target, result, kind
            ).red());
        }
    }

    pub fn run_ruleset(&self, context: &Context, ruleset: Option<&Ruleset>, path: &Path, is_metric: bool)
    -> Result<toml::value::Table,()>
    {
        let mut scores = toml::value::Table::default();

        if ruleset.is_none() {
            println!("{}", "No targets.".yellow());
            return Ok(scores)
        }

        let ruleset = ruleset.unwrap();

        let count = ruleset.rules.len();
        let mut passed = 0usize;
        let mut failed = 0usize;
        let mut fatal = false;

        for mut rule in ruleset.rules.iter().cloned() {
            rule.fail_okay.get_or_insert(ruleset.fail_okay.unwrap_or(false));
            util::print_hline();
            let mut did_pass: bool = false;

            match self.run_rule(context, &rule, path) {
                Ok(pass) => {
                    if pass {
                        passed += 1;
                        did_pass = true;
                    } else {
                        failed += 1;
                    }
                },
                Err(()) => {
                    failed += 1;
                    fatal = true;
                    break;
                }
            }

            if did_pass && is_metric {
                let result = fs::read_to_string(path.join(&rule.target))
                    .map_err(|err| FailInfo::IOFail(format!("Failed to read score : '{}'",err)).into_log());
                match (rule.kind.as_ref(), result) {
                    (Some(kind),Ok(result)) => Self::log_metric(&mut scores,&rule.target,&result,&kind),
                    (None,Ok(_result)) => {
                        println!("{}",format!("! Metric '{}' has no kind.",&rule.target).red());
                    }
                    (_,Err(log))   => print!("{}",log),
                }
            }
        }
        if fatal {
            println!("{}",format!("! {}","Execution cannot continue beyond this error.").red());
        }
        util::print_hline();
        let not_reached = count-passed-failed;
        println!("! {count} total targets - {passed} passed, {failed} failed, {not_reached} not reached.");

        if fatal {
            return Err(());
        }

        Ok(scores)
    }


    pub fn run_on_submit(&self, context: &Context, ruleset : Option<&Ruleset>, path: &Path, title: &str, is_metric: bool)
    -> Option<Result<toml::value::Table,()>>
    {
        if let Some(set) = ruleset {
            if set.on_submit.unwrap_or(true) {
                util::print_bold_hline();
                println!("{}",title.yellow().bold());
                return Some(self.run_ruleset(context,Some(set),path,is_metric));
            }
        }
        return None;
    }

    pub fn run_on_grade(&self, context: &Context, ruleset : Option<&Ruleset>, path: &Path, title: &str, is_metric: bool)
    -> Option<Result<toml::value::Table,()>>
    {
        if let Some(set) = ruleset {
            if set.on_grade.unwrap_or(true) {
                util::print_bold_hline();
                println!("{}",title.yellow().bold());
                return Some(self.run_ruleset(context,Some(set),path,is_metric));
            }
        }
        return None;
    }

    pub fn retrieve_sub(&self, dst_dir : &Path, user_name : &str)
    -> Result<(),FailLog>
    {
        let sub_path = self.path.join(user_name);

        if dst_dir.is_dir() {
            fs::remove_dir_all(&dst_dir).map_err(|err|
                FailInfo::IOFail(format!("could not remove directory {}: {}", dst_dir.display(), err)).into_log()
            )?;
        }
        if dst_dir.is_file() {
            fs::remove_file(&dst_dir).map_err(|err|
                FailInfo::IOFail(format!("could not remove file {}: {}", dst_dir.display(), err)).into_log()
            )?;
        }
        fs::create_dir(&dst_dir).map_err(|err|
            FailInfo::IOFail(format!("could not create directory {}: {}", dst_dir.display(), err)).into_log()
        )?;

        for file_name in self.file_list.iter() {
            let src_path = sub_path.join(file_name);
            let dst_path = dst_dir .join(file_name);
            if src_path.is_dir() {
                continue;
            }

            if ! src_path.exists() {
                return Err(FailInfo::Custom(
                    format!("could not copy file {} to {}", (&src_path).display(), (&dst_path).display()),
                    format!("File does not exist in the submission directory.")
                ).into_log());
            }

            fs::copy(&src_path,&dst_path).map_err(|err|
                FailInfo::IOFail(
                    format!("could not copy file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)
                ).into_log()
            )?;
        }

        Ok(())
    }

}


pub struct SubmissionSlot <'ctx> {
    pub context   : &'ctx Context,
    pub asgn_spec : &'ctx AsgnSpec,
    pub base_path : PathBuf,
}

pub struct SubmissionStatus {
    pub turn_in_time   : Option<DateTime<Local>>,
    pub grace_days     : i64,
    pub extension_days : i64,
}

#[derive(Serialize, Deserialize)]
struct GraceToml {
    pub value : i64,
}

#[derive(Serialize, Deserialize)]
struct ExtensionToml {
    pub value : i64,
}


impl <'ctx> SubmissionSlot<'ctx> {
    pub fn grace_path(&self) -> PathBuf {
        self.base_path.join(".grace")
    }

    pub fn extension_path(&self) -> PathBuf {
        self.base_path.join(".extension")
    }

    pub fn file_paths<'a>(&'a self) -> impl 'a + Iterator<Item=PathBuf> {
        self.asgn_spec.file_list.iter()
            .map(|name| self.base_path.join(name))
    }

    pub fn get_grace(&self) -> Result<i64, FailLog> {
        let toml_text = fs::read_to_string(self.grace_path()).map_err(|err|
            FailInfo::IOFail(format!("reading grace file: {}",err)).into_log()
        );
        if toml_text.is_err() {
            return Ok(0);
        }
        let grace : GraceToml = toml::from_str(&toml_text.unwrap()).map_err(|err|
            FailInfo::IOFail(format!("deserializing grace file: {}",err)).into_log()
        )?;
        Ok(grace.value)
    }

    pub fn set_grace(&self, value: i64) -> Result<(),FailLog> {
        let grace_toml = GraceToml { value };
        let toml_text  = toml::to_string(&grace_toml).map_err(|err|
            FailInfo::IOFail(format!("serializing grace file : {}",err)).into_log()
        )?;
        fs::write(self.grace_path(),toml_text).map_err(|err|
            FailInfo::IOFail(format!("writing grace file : {}",err)).into_log()
        )?;
        Ok(())
    }

    pub fn get_extension(&self) -> Result<i64,FailLog> {
        let ext_path = self.extension_path();

        if ! ext_path.exists() {
            return Ok(0);
        }
        if ext_path.is_dir() {
            return Ok(0);
        }

        let owner_uid = std::fs::metadata(&ext_path)
            .map_err(|err| FailInfo::IOFail(err.to_string()))?.uid();
        let owner : OsString = get_user_by_uid(owner_uid)
            .ok_or(FailInfo::InvalidUID() )?.name().into();

        if owner != self.context.instructor {
            return Err(FailInfo::IOFail(format!(
                "Extension file at {} was not made by instructor!",
                ext_path.display()
            )).into());
        }

        let toml_text = fs::read_to_string(ext_path).map_err(|err|
            FailInfo::IOFail(format!("reading extension file: {err}")).into_log()
        )?;

        let ext: ExtensionToml = toml::from_str(&toml_text).map_err(|err|
            FailInfo::IOFail(format!("deserializing extension file: {err}")).into_log()
        )?;

        Ok(ext.value)
    }

    pub fn set_extension(&self, value: i64) -> Result<(),FailLog> {
        let ext_toml = ExtensionToml { value };
        let toml_text  = toml::to_string(&ext_toml).map_err(|err|
            FailInfo::IOFail(format!("serializing extension file : {}",err)).into_log()
        )?;
        fs::write(self.extension_path(),toml_text).map_err(|err|
            FailInfo::IOFail(format!("writing extension file : {}",err)).into_log()
        )?;
        Ok(())
    }

    pub fn status(&self) -> Result<SubmissionStatus,FailLog> {
        let submitted = self.file_paths().all(|p| p.is_file());

        let time: Option<i64> = if submitted {
            let mut mtime: i64 = 0;
            for path in self.file_paths().into_iter() {
                let meta = fs::metadata(path).map_err(|err|{
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
            grace_days: self.get_grace()?,
            extension_days: self.get_extension()?,
        })
    }
}


impl SubmissionStatus {
    pub fn time_past(&self, time: &DateTime<Local>) -> Option<Duration> {
        Some(self.turn_in_time?.signed_duration_since(time))
    }

    pub fn versus(&self, time: Option<&DateTime<Local>>) -> String {
        let Some(time) = time else {
            if self.turn_in_time.is_some() {
                return String::from("Submitted");
            } else {
                return String::from("Not Submitted");
            }
        };

        let late_by = self.time_past(time);

        if late_by.is_none() {
            let time_diff = chrono::offset::Local::now()
                .signed_duration_since(*time);
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

impl StatBlockSet {
    pub fn get_block(&self, user: &str) -> Option<&StatBlock> {
        self.stat_block.iter().flatten().find(|block| block.user == user)
    }
}
