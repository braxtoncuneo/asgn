use std::path::{Path, PathBuf};

use crate:: {
    context::Context,
    error:: {ErrorLog, Error},
    asgn_spec::{AsgnSpec, StatBlock, StatBlockSet},
    act::{student::StudentAct, grader::GraderAct},
    util::{
        self,
        color::{FG_YELLOW, TEXT_BOLD, STYLE_RESET},
        TomlDatetimeExt,
        ChronoDateTimeExt,
    },
};

use structopt::StructOpt;
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
    Refresh,
}

impl InstructorAct {
    fn add_students(usernames: Vec<String>, context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            for username in usernames {
                if !ctx.students.iter().any(|student| student == &username) {
                    ctx.students.push(username);
                }
            }
        )?;
        context.refresh()
    }

    fn remove_students(usernames: &[String], context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            ctx.students.retain(|student| !usernames.contains(student))
        )
    }

    fn add_graders(usernames: Vec<String>, context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            for username in usernames {
                if !ctx.graders.iter().any(|grader| grader == &username) {
                    ctx.graders.push(username);
                }
            }
        )?;
        context.refresh()
    }

    fn remove_graders(usernames: &[String], context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            ctx.graders.retain(|grader| !usernames.contains(grader))
        )
    }

    fn add_assignments(asgn_names: Vec<String>, context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            for asgn_name in asgn_names {
                if !ctx.manifest.iter().any(|assignment| assignment == &asgn_name) {
                    ctx.manifest.push(asgn_name);
                }
            }
        )?;
        context.refresh()?;
        context.populate_catalog();
        Ok(())
    }

    fn remove_assignments(asgn_names: &[String], context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            ctx.manifest.retain(|name| !asgn_names.contains(name))
        )
    }

    fn set_due(asgn_name: &str, date: &str, context: &mut Context) -> Result<(), Error> {
        let date = util::parse_toml_date_as_chrono(date)?;

        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.due_date = Some(date)
        )
    }

    fn set_open(asgn_name: &str, date: &str, context: &mut Context) -> Result<(), Error> {
        let date = util::parse_toml_date_as_chrono(date)?;

        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.open_date = Some(date)
        )
    }

    fn set_close(asgn_name: &str, date: &str, context: &mut Context) -> Result<(), Error> {
        let date = util::parse_toml_date_as_chrono(date)?;

        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.close_date = Some(date)
        )
    }

    fn unset_due(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.due_date = None
        )
    }

    fn unset_open(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.open_date = None
        )
    }

    fn unset_close(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.close_date = None
        )
    }

    fn grace_total(total: i64, context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            ctx.grace_total = Some(total)
        )
    }

    fn grace_limit(limit: i64, context: &mut Context) -> Result<(), Error> {
        context.modify_synced(|ctx|
            ctx.grace_limit = Some(limit)
        )
    }

    fn publish(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.visible = true
        )
    }

    fn unpublish(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.visible = false
        )
    }

    fn enable(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.active = true
        )
    }

    fn disable(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        context.catalog_get_mut(asgn_name)?.modify_synced(|spec|
            spec.active = false
        )
    }

    fn extend(asgn_name: &str, username: &str, ext_days: i64, context : &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;
        let slot = context.get_slot(spec, username);
        slot.set_extension(ext_days)
    }

    fn latest_score(
        old_stats: &StatBlockSet,
        username: &str,
        build_root: &Path,
        asgn: &AsgnSpec,
        context: &Context
    ) -> Result<Option<StatBlock>, Error>
    {
        let Some(turn_in_time) = context
            .get_slot(asgn, username)
            .status().unwrap()
            .turn_in_time
        else {
            return Ok(None);
        };

        if let Some(stats) = old_stats.get_block(username) {
            let old_time = stats.time.try_into_chrono_date_time().ok_or_else(||
                Error::bad_stats(username, "Missing date")
            )?;

            if turn_in_time.signed_duration_since(old_time) <= Duration::seconds(1) {
                println!("{FG_YELLOW}{TEXT_BOLD}{username} is already up-to-date.{STYLE_RESET}");
                return Ok(Some(stats.clone()));
            }
        }

        let build_path = build_root.join(username);
        asgn.retrieve_sub(&build_path, username)?;
        if !build_root.exists() {
            println!("{} does not exist!", build_root.display());
        }
        if !build_path.exists() {
            println!("{} does not exist!", build_path.display());
        }
        let _ = asgn.run_ruleset(context, asgn.build.as_ref(), &build_path, false);

        let scores = asgn.run_ruleset(context, asgn.score.as_ref(), &build_path, true).unwrap_or_default();

        Ok(Some(StatBlock {
            username: username.to_owned(),
            time: turn_in_time.to_toml_datetime(),
            scores
        }))
    }

    fn update_scores(asgn_name: &str, context: &mut Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;
        let info_path = spec.path.join(".info");
        let stat_path = &info_path.join("score.toml");

        let old_stats: StatBlockSet = util::parse_toml_file(stat_path)?;
        let new_stats: StatBlockSet = {
            let build_path = info_path.join(".internal").join("score_build");
            let build_dir = tempdir_in(&build_path).map_err(|err|
                Error::io("Failed to create temp dir", build_path, err)
            )?;

            context.members.iter().filter_map(|member| {
                let score = Self::latest_score(&old_stats, member, build_dir.path(), spec, context);
                match score {
                    Ok(Some(score)) => Some(score),
                    Ok(None) => {
                        println!("{FG_YELLOW}{TEXT_BOLD}{member} has no submission.{STYLE_RESET}");
                        None
                    }
                    Err(err) => {
                        print!("{err}");
                        None
                    }
                }
            }).collect()
        };

        util::write_toml_file(&new_stats, stat_path)
    }

    fn update_all_scores(context: &mut Context) -> Result<(), ErrorLog> {
        let ok_asgn: Vec<_> = context.manifest.iter()
            .filter_map(|name| context.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok())
            .map(|asgn| asgn.name.clone())
            .collect();

        let log: ErrorLog = ok_asgn.into_iter()
            .map(|asgn| Self::update_scores(&asgn, context))
            .filter_map(Result::err)
            .collect();

        log.into_result()
    }

    pub fn execute(self, context: &mut Context) -> Result<(), ErrorLog> {
        use InstructorAct::*;
        match self {
            Grader          (act) => act.execute(context)?,
            ListAsgns       {} => context.list_asgns()?,
            ListSubs        { asgn_name, username } => context.list_subs(
                asgn_name.as_ref().unwrap_or(&None).as_deref(),
                username.as_ref().unwrap_or(&None).as_deref(),
            )?,
            AddStudents     { usernames       } => Self::add_students(usernames, context)?,
            RemStudents     { usernames       } => Self::remove_students(&usernames, context)?,
            AddGraders      { usernames       } => Self::add_graders(usernames, context)?,
            RemGraders      { usernames       } => Self::remove_graders(&usernames, context)?,
            AddAsgns        { asgn_names      } => Self::add_assignments(asgn_names, context)?,
            RemAsgns        { asgn_names      } => Self::remove_assignments(&asgn_names, context)?,
            SetDue          { asgn_name, date } => Self::set_due(&asgn_name, &date, context)?,
            SetOpen         { asgn_name, date } => Self::set_open(&asgn_name, &date, context)?,
            SetClose        { asgn_name, date } => Self::set_close(&asgn_name, &date, context)?,
            UnsetDue        { asgn_name       } => Self::unset_due(&asgn_name, context)?,
            UnsetOpen       { asgn_name       } => Self::unset_open(&asgn_name, context)?,
            UnsetClose      { asgn_name       } => Self::unset_close(&asgn_name, context)?,
            Publish         { asgn_name       } => Self::publish(&asgn_name, context)?,
            Unpublish       { asgn_name       } => Self::unpublish(&asgn_name, context)?,
            Enable          { asgn_name       } => Self::enable(&asgn_name, context)?,
            Disable         { asgn_name       } => Self::disable(&asgn_name, context)?,
            UpdateScores    { asgn_name       } => Self::update_scores(&asgn_name, context)?,
            UpdateAllScores {                 } => Self::update_all_scores(context)?,
            GraceTotal      { num             } => Self::grace_total(num, context)?,
            GraceLimit      { num             } => Self::grace_limit(num, context)?,
            Refresh         {                 } => context.refresh()?,
            Extend          { asgn_name, username, ext } => Self::extend(&asgn_name, &username, ext, context)?,
            SetGrace        { asgn_name, username, ext } => StudentAct::grace(&asgn_name, &username, ext, context)?,
        }

        Ok(())
    }
}
