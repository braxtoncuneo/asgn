use std::{
    str::FromStr,
    path::{Path, PathBuf},
};

use crate:: {
    context::Context,
    fail_info:: {FailLog, FailInfo},
    asgn_spec::{AsgnSpec, StatBlock, StatBlockSet},
    act::{student::StudentAct, grader::GraderAct},
    util,
};

use structopt::StructOpt;
use colored::Colorize;
use tempfile::tempdir_in;
use chrono::Duration;

#[derive(Debug, StructOpt)]
#[structopt(
    name       = "asgn - instructor version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct InstructorCmd {
    #[structopt(name = "base path")]
    _base_path: PathBuf, // Used only to consume the first CLI arg

    #[structopt(subcommand)]
    pub act: InstructorAct,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "snake")]
pub enum InstructorAct {
    #[structopt(flatten)]
    Grader(GraderAct),

    // Instructors Only
    #[structopt(about = "[instructors only] adds the listed students to the course's student list")]
    AddStudents {
        #[structopt(name = "usernames")]
        usernames: Vec<String>,
    },

    #[structopt(about = "[instructors only] removes the listed students from the course's student list")]
    RemStudents {
        #[structopt(name = "usernames")]
        usernames: Vec<String>,
    },

    #[structopt(about = "[instructors only] adds the listed graders to the course's grader list")]
    AddGraders {
        #[structopt(name = "usernames")]
        usernames: Vec<String>,
    },

    #[structopt(about = "[instructors only] removes the listed graders from the course's grader list")]
    RemGraders {
        #[structopt(name = "usernames")]
        usernames: Vec<String>,
    },

    #[structopt(about = "[instructors only] adds the listed assignments to the course manifest, initialized to a blank assignment")]
    AddAsgns {
        #[structopt(name = "assignment names")]
        asgn_names: Vec<String>,
    },

    #[structopt(about = "[instructors only] removes the listed assigments from the course manifest")]
    RemAsgns {
        #[structopt(name = "usernames")]
        asgn_names: Vec<String>,
    },

    #[structopt(about = "[instructors only] lists the course's assignments")]
    ListAsgns {},

    #[structopt(about = "[instructors only] lists the course's current submissions")]
    ListSubs {
        #[structopt(name = "assignment name", long = "asgn")]
        asgn_name:  Option<Option<String>>,
        #[structopt(name = "student username", long = "user")]
        username: Option<Option<String>>,
    },

    #[structopt(about = "[instructors only] sets the due date of an assignment")]
    SetDue {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "yyyy-mm-dd")]
        date: String,
    },

    #[structopt(about = "[instructors only] sets the open date of an assignment")]
    SetOpen {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "yyyy-mm-dd")]
        date: String,
    },

    #[structopt(about = "[instructors only] sets the close date of an assignment")]
    SetClose {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "yyyy-mm-dd")]
        date: String,
    },

    #[structopt(about = "[instructors only] removes the due date of an assignment")]
    UnsetDue {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] removes the open date of an assignment")]
    UnsetOpen {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] removes the close date of an assignment")]
    UnsetClose {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] enables assignment, allowing setup, submission, etc")]
    Enable {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] disables assignment, disallowing setup, submission, etc")]
    Disable {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] publishes the assignment, allowing students to see it in the course summary")]
    Publish {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] unpublishes the assignment, disallowing students from seeing it in the course summary")]
    Unpublish {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] updates published scores for a given assignment based upon current submissions")]
    UpdateScores {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] updates published scores for all assignments based upon current submissions")]
    UpdateAllScores {},

    /*
    #[structopt(about = "[instructors only] checks an assignment specification for validity")]
    Audit {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "[instructors only] checks all assignment specifications for validity")]
    AuditAll {},
    */

    #[structopt(about = "[instructors only] assigns an integer-day extension to a particular user for a particular assignment")]
    Extend {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "username")]
        username: String,
        #[structopt(name = "extension amount")]
        ext : i64,
    },

    #[structopt(about = "[instructors only] assigns an integer-day number of grace days to a particular user for a particular assignment")]
    SetGrace {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "username")]
        username: String,
        #[structopt(name = "grace amount")]
        ext : i64,
    },

    #[structopt(about = "[instructors only] sets the total number of grace days students may use")]
    GraceTotal {
        #[structopt(name = "grace day total")]
        num: i64,
    },

    #[structopt(about = "[instructors only] sets the maximum number of grace days students may use on an assignment")]
    GraceLimit {
        #[structopt(name = "per-assignment grace day limit")]
        num: i64,
    },

    #[structopt(about = "[instructors only] attempts to fix the state of the course directory")]
    Refresh {},
}

#[allow(dead_code)]
impl InstructorAct {
    fn add_students(usernames: Vec<String>, context: &mut Context) -> Result<(), FailLog> {
        for username in usernames {
            if !context.students.iter().any(|student| student == &username) {
                context.students.push(username);
            }
        }
        context.sync()?;
        context.refresh()
    }

    fn remove_students(usernames: &[String], context: &mut Context) -> Result<(), FailLog> {
        context.students.retain(|student| !usernames.contains(student));
        context.sync()
    }

    fn add_graders(usernames: Vec<String>, context: &mut Context) -> Result<(), FailLog> {
        for username in usernames {
            if !context.graders.iter().any(|grader| grader == &username) {
                context.graders.push(username);
            }
        }
        context.sync()?;
        context.refresh()
    }

    fn remove_graders(usernames: &[String], context: &mut Context) -> Result<(), FailLog> {
        context.graders.retain(|grader| !usernames.contains(grader));
        context.sync()
    }

    fn add_assignments(asgn_names: Vec<String>, context: &mut Context) -> Result<(), FailLog> {
        for asgn_name in asgn_names {
            if !context.manifest.iter().any(|assignment| assignment == &asgn_name) {
                context.manifest.push(asgn_name);
            }
        }
        context.sync()?;
        context.refresh()?;
        context.populate_catalog();
        Ok(())
    }

    fn remove_assignments(asgn_names: &[String], context: &mut Context) -> Result<(), FailLog> {
        context.manifest.retain(|name| !asgn_names.contains(name));
        context.sync()
    }

    fn set_due(asgn_name: &str, date: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        let date = toml::value::Datetime::from_str(date).map_err(|err|
            FailInfo::IOFail(err.to_string())
        )?;
        spec.due_date = Some(util::date_into_chrono(date)?);
        spec.sync()
    }

    fn set_open(asgn_name: &str, date: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        let date = toml::value::Datetime::from_str(date).map_err(|err|
            FailInfo::IOFail(err.to_string())
        )?;
        spec.open_date = Some(util::date_into_chrono(date)?);
        spec.sync()
    }

    fn set_close(asgn_name: &str, date: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        let date = toml::value::Datetime::from_str(date).map_err(|err|
            FailInfo::IOFail(err.to_string())
        )?;
        spec.close_date = Some(util::date_into_chrono(date)?);
        spec.sync()
    }

    fn unset_due(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        spec.due_date = None;
        spec.sync()
    }

    fn unset_open(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        spec.open_date = None;
        spec.sync()
    }

    fn unset_close(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec= context.catalog_get_mut(asgn_name)?;
        spec.close_date = None;
        spec.sync()
    }

    fn grace_total(total: i64, context: &mut Context) -> Result<(), FailLog> {
        context.grace_total = Some(total);
        context.sync()
    }

    fn grace_limit(limit: i64, context: &mut Context) -> Result<(), FailLog> {
        context.grace_limit = Some(limit);
        context.sync()
    }

    fn no_grace(context: &mut Context) -> Result<(), FailLog> {
        context.grace_total = None;
        context.grace_limit = None;
        context.sync()
    }

    fn publish(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        spec.visible = true;
        spec.sync()
    }

    fn unpublish(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        spec.visible = false;
        spec.sync()
    }

    fn enable(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        spec.active = true;
        spec.sync()
    }

    fn disable(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get_mut(asgn_name)?;
        spec.active = false;
        spec.sync()
    }


    fn extend(asgn_name: &str, username: &str, ext_days: i64, context : &Context) -> Result<(), FailLog> {
        let spec: &AsgnSpec = context.catalog_get(asgn_name)?;
        let slot = context.get_slot(spec, username);
        slot.set_extension(ext_days)
    }

    fn latest_score(old_stats: &StatBlockSet, username: &str, build_root: &Path, asgn: &AsgnSpec, context: &Context)
    -> Result<Option<StatBlock>, FailLog>
    {

        let slot = context.get_slot(asgn, username);
        let status = slot.status().unwrap();

        let Some(turn_in_time) = status.turn_in_time else {
            return Ok(None);
        };

        let stats = old_stats.get_block(username);

        if let Some(stats) = stats {
            let old_time = util::date_into_chrono(stats.time.clone())?;
            if turn_in_time.signed_duration_since(old_time) <= Duration::seconds(1) {
                println!("{}", format!("{username} is already up-to-date.").yellow().bold());
                return Ok(Some(stats.clone()));
            }
        }

        let build_path = build_root.join(username);
        asgn.retrieve_sub(&build_path, username)?;
        if ! build_root.exists() {
            println!("{} does not exist!", build_root.display());
        }
        if ! build_path.exists() {
            println!("{} does not exist!", build_path.display());
        }
        let _ = asgn.run_ruleset(context, asgn.build.as_ref(), &build_path, false);

        let time = util::date_from_chrono(turn_in_time);
        let scores = asgn.run_ruleset(context, asgn.score.as_ref(), &build_path, true).unwrap_or_default();

        let stat_block = StatBlock {
            username: username.to_owned(),
            time,
            scores
        };

        Ok(Some(stat_block))
    }

    fn update_scores(asgn_name: &str, context: &mut Context) -> Result<(), FailLog> {
        let spec = context.catalog_get(asgn_name)?;
        let info_path = spec.path.join(".info");
        let build_path = info_path.join(".internal").join("score_build");
        let build_path = tempdir_in(build_path.clone()).map_err(|err|
            FailInfo::IOFail(format!("Failed to create temp dir '{}': {}", build_path.display(), err)).into_log()
        )?;

        let stat_path = info_path.join("score.toml");
        let old_stats = util::parse_from::<StatBlockSet>(&stat_path)?;

        let mut new_stats: StatBlockSet = Default::default();

        for member in &context.members {
            match Self::latest_score(&old_stats, member, build_path.path(), spec, context) {
                Ok(Some(block)) => if new_stats.stat_block.is_some() {
                    new_stats.stat_block.as_mut().unwrap().push(block);
                } else {
                    new_stats.stat_block = Some(vec![block]);
                }
                Err(log) => print!("{log}"),
                _ => println!("{}", format!("{member} has no submission.").yellow().bold()),
            }
        }

        util::serialize_into(&stat_path, &new_stats)?;

        Ok(())
    }

    fn update_all_scores(context: &mut Context) -> Result<(), FailLog> {
        let ok_asgn: Vec<_> = context.manifest.iter()
            .filter_map(|name| context.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok())
            .map(|asgn| asgn.name.clone())
            .collect();

        let log: FailLog = ok_asgn.into_iter()
            .map(|asgn| Self::update_scores(&asgn, context))
            .filter_map(Result::err)
            .flatten()
            .collect();

        if !log.is_empty() {
            print!("{log}");
        }

        Ok(())
    }

    pub fn execute(self, context: &mut Context) -> Result<(), FailLog> {
        use InstructorAct::*;
        match self {
            Grader          (act) => act.execute(context),
            ListAsgns       {} => context.list_asgns(),
            ListSubs        { asgn_name, username } => context.list_subs(
                asgn_name.as_ref().unwrap_or(&None).as_deref(),
                username.as_ref().unwrap_or(&None).as_deref(),
            ),
            AddStudents     { usernames      } => Self::add_students(usernames, context),
            RemStudents     { usernames      } => Self::remove_students(&usernames, context),
            AddGraders      { usernames      } => Self::add_graders(usernames, context),
            RemGraders      { usernames      } => Self::remove_graders(&usernames, context),
            AddAsgns        { asgn_names      } => Self::add_assignments(asgn_names, context),
            RemAsgns        { asgn_names      } => Self::remove_assignments(&asgn_names, context),
            SetDue          { asgn_name, date } => Self::set_due(&asgn_name, &date, context),
            SetOpen         { asgn_name, date } => Self::set_open(&asgn_name, &date, context),
            SetClose        { asgn_name, date } => Self::set_close(&asgn_name, &date, context),
            UnsetDue        { asgn_name       } => Self::unset_due(&asgn_name, context),
            UnsetOpen       { asgn_name       } => Self::unset_open(&asgn_name, context),
            UnsetClose      { asgn_name       } => Self::unset_close(&asgn_name, context),
            Publish         { asgn_name       } => Self::publish(&asgn_name, context),
            Unpublish       { asgn_name       } => Self::unpublish(&asgn_name, context),
            Enable          { asgn_name       } => Self::enable(&asgn_name, context),
            Disable         { asgn_name       } => Self::disable(&asgn_name, context),
            UpdateScores    { asgn_name       } => Self::update_scores(&asgn_name, context),
            UpdateAllScores {                 } => Self::update_all_scores(context),
            GraceTotal      { num             } => Self::grace_total(num, context),
            GraceLimit      { num             } => Self::grace_limit(num, context),
            Refresh         {                 } => context.refresh(),
            Extend          { asgn_name, username, ext } => Self::extend(&asgn_name, &username, ext, context),
            SetGrace        { asgn_name, username, ext } => StudentAct::grace(&asgn_name, &username, ext, context),
        }
    }
}
