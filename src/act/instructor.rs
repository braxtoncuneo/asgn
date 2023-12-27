use std::{
    ffi::OsString,
    str::FromStr,
    path::Path, iter,
};


use crate::
{
    context::Context,
    fail_info::
    {
        FailLog,
        FailInfo,
    },
    asgn_spec::{
        AsgnSpec,
        StatBlock,
        StatBlockSet,
    },
    act::{
        student::StudentAct,
        grader::GraderAct,
    },
    util,
};

use structopt::StructOpt;
use colored::Colorize;
use tempfile::tempdir_in;
use chrono::Duration;

#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn - instructor version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct InstructorCmd
{

    #[structopt(name = "base path")]
    base_path : OsString,

    #[structopt(subcommand)]
    pub act: InstructorAct,
}



#[derive(Debug,StructOpt)]
#[structopt(rename_all = "snake")]
pub enum InstructorAct
{
    #[structopt(flatten)]
    Grader(GraderAct),

    // Instructors Only
    #[structopt(about = "[instructors only] adds the listed students to the course's student list")]
    AddStudents
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] removes the listed students from the course's student list")]
    RemStudents
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] adds the listed graders to the course's grader list")]
    AddGraders
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] removes the listed graders from the course's grader list")]
    RemGraders
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] adds the listed assignments to the course manifest, initialized to a blank assignment")]
    AddAsgns
    {
        #[structopt(name = "assignment names")]
        asgn_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] removes the listed assigments from the course manifest")]
    RemAsgns
    {
        #[structopt(name = "user names")]
        asgn_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] lists the course's assignments")]
    ListAsgns{},
    #[structopt(about = "[instructors only] lists the course's current submissions")]
    ListSubs
    {
        #[structopt(name = "assignment name",long = "asgn")]
        asgn:  Option<Option<OsString>>,
        #[structopt(name = "student user name",long = "user")]
        user : Option<Option<OsString>>,
    },
    #[structopt(about = "[instructors only] sets the due date of an assignment")]
    SetDue
    {
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "yyyy-mm-dd")]
        date: OsString,
    },
    #[structopt(about = "[instructors only] sets the open date of an assignment")]
    SetOpen{
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "yyyy-mm-dd")]
        date: OsString,
    },
    #[structopt(about = "[instructors only] sets the close date of an assignment")]
    SetClose{
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "yyyy-mm-dd")]
        date: OsString,
    },
    #[structopt(about = "[instructors only] removes the due date of an assignment")]
    UnsetDue{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] removes the open date of an assignment")]
    UnsetOpen{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] removes the close date of an assignment")]
    UnsetClose{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] enables assignment, allowing setup, submission, etc")]
    Enable{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] disables assignment, disallowing setup, submission, etc")]
    Disable{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] publishes the assignment, allowing students to see it in the course summary")]
    Publish{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] unpublishes the assignment, disallowing students from seeing it in the course summary")]
    Unpublish{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] updates published scores for a given assignment based upon current submissions")]
    UpdateScores{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
    #[structopt(about = "[instructors only] updates published scores for all assignments based upon current submissions")]
    UpdateAllScores{},
    /*
    #[structopt(about = "[instructors only] checks an assignment specification for validity")]
    Audit
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },

    #[structopt(about = "[instructors only] checks all assignment specifications for validity")]
    AuditAll{},
    */
    #[structopt(about = "[instructors only] assigns an integer-day extension to a particular user for a particular assignment")]
    Extend
    {
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "user name")]
        user: OsString,
        #[structopt(name = "extension amount")]
        ext : i64,
    },
    #[structopt(about = "[instructors only] assigns an integer-day number of grace days to a particular user for a particular assignment")]
    SetGrace
    {
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "user name")]
        user: OsString,
        #[structopt(name = "grace amount")]
        ext : i64,
    },
    #[structopt(about = "[instructors only] sets the total number of grace days students may use")]
    GraceTotal
    {
        #[structopt(name = "grace day total")]
        num: i64,
    },
    #[structopt(about = "[instructors only] sets the maximum number of grace days students may use on an assignment")]
    GraceLimit
    {
        #[structopt(name = "per-assignment grace day limit")]
        num: i64,
    },
    #[structopt(about = "[instructors only] attempts to fix the state of the course directory")]
    Refresh{},

}



impl InstructorAct
{

    fn add_students(user_names: Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        for name in user_names.iter() {
            if ! context.students.contains(&mut name.clone()) {
                context.students.push(name.clone());
            }
        }
        context.sync()?;
        context.refresh()
    }

    fn remove_students(user_names: Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        context.students.retain(|os_str| ! user_names.contains(os_str) );
        context.sync()
    }

    fn add_graders(user_names: Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        for name in user_names.iter() {
            if ! context.graders.contains(&mut name.clone()) {
                context.graders.push(name.clone());
            }
        }
        context.sync()?;
        context.refresh()
    }

    fn remove_graders(user_names: Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        context.graders.retain(|os_str| ! user_names.contains(os_str) );
        context.sync()
    }

    fn add_assignments(asgn_names: Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        for name in asgn_names.iter() {
            if ! context.manifest.contains(&mut name.clone()) {
                context.manifest.push(name.clone());
            }
        }
        context.sync()?;
        context.refresh()?;
        context.populate_catalog();
        Ok(())
    }

    fn remove_assignments(asgn_names: Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        context.manifest.retain(|os_str| ! asgn_names.contains(os_str) );
        context.sync()
    }

    fn get_mut_spec<'a>(asgn: &OsString, context: &'a mut Context) -> Result<&'a mut AsgnSpec,FailLog>
    {
        context.catalog.get_mut(asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone())
    }

    fn set_due(asgn: OsString, date: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        let date = toml::value::Datetime::from_str(&date.to_string_lossy())
            .map_err(|err| FailInfo::IOFail(format!("{}",err)))?;
        spec.due_date = Some(util::date_into_chrono(date)?);
        spec.sync()
    }

    fn set_open(asgn: OsString, date: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        let date = toml::value::Datetime::from_str(&date.to_string_lossy())
            .map_err(|err| FailInfo::IOFail(format!("{}",err)))?;
        spec.open_date = Some(util::date_into_chrono(date)?);
        spec.sync()
    }

    fn set_close(asgn: OsString, date: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        let date = toml::value::Datetime::from_str(&date.to_string_lossy())
            .map_err(|err| FailInfo::IOFail(format!("{}",err)))?;
        spec.close_date = Some(util::date_into_chrono(date)?);
        spec.sync()
    }

    fn unset_due(asgn: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        spec.due_date = None;
        spec.sync()
    }

    fn unset_open(asgn: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        spec.open_date = None;
        spec.sync()
    }

    fn unset_close(asgn: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        spec.close_date = None;
        spec.sync()
    }

    fn grace_total(total: i64, context: &mut Context) -> Result<(),FailLog>
    {
        context.grace_total = Some(total);
        context.sync()
    }

    fn grace_limit(limit: i64, context: &mut Context) -> Result<(),FailLog>
    {
        context.grace_limit = Some(limit);
        context.sync()
    }

    fn no_grace(context: &mut Context) -> Result<(),FailLog>
    {
        context.grace_total = None;
        context.grace_limit = None;
        context.sync()
    }

    fn publish(asgn: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.visible = true;
        spec.sync()
    }

    fn unpublish(asgn: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.visible = false;
        spec.sync()
    }

    fn enable(asgn: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.active = true;
        spec.sync()
    }

    fn disable(asgn: OsString, context : &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.active = false;
        spec.sync()
    }


    fn extend(asgn: OsString, user: OsString, ext_days: i64, context : &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        let slot = context.get_slot(spec,&user);
        slot.set_extension(ext_days)
    }

    fn latest_score(old_stats : &StatBlockSet, user : &OsString, build_root: &Path, asgn: &AsgnSpec, context : &Context)
     -> Result<Option<StatBlock>,FailLog>
    {

        let slot = context.get_slot(asgn,user);
        let status = slot.status().unwrap();

        let Some(turn_in_time) = status.turn_in_time else {
            return Ok(None);
        };

        let user_name = user.clone().into_string().unwrap();
        let stats = old_stats.get_block(&user_name);

        if let Some(stats) = stats {
            let old_time = util::date_into_chrono(stats.time.clone())?;
            if turn_in_time.signed_duration_since(old_time) <= Duration::seconds(1) {
                println!("{}",format!(
                    "{} is already up-to-date.",
                    user.clone().into_string().unwrap()
                ).yellow().bold());
                return Ok(Some(stats.clone()));
            }
        }

        let build_path = build_root.join(&user);
        asgn.retrieve_sub(&build_path,&user.clone().into_string().unwrap())?;
        if ! build_root.exists() {
            println!("{} does not exist!",build_root.display());
        }
        if ! build_path.exists() {
            println!("{} does not exist!",build_path.display());
        }
        let _ = asgn.run_ruleset(context,asgn.build.as_ref(),&build_path,false);

        let user   = user.clone().into_string().unwrap();
        let time   = util::date_from_chrono(turn_in_time);
        let scores = asgn.run_ruleset(context,asgn.score.as_ref(),&build_path,true)
            .ok()
            .unwrap_or(Default::default());

        let stat_block = StatBlock {
            user,
            time,
            scores
        };

        Ok(Some(stat_block))
    }


    fn update_scores(asgn: OsString, context : &mut Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        let info_path  = spec.path.join(".info");
        let build_path = info_path.join(".internal").join("score_build");
        let build_path = tempdir_in(build_path.clone())
            .map_err(|err|FailInfo::IOFail(
                format!("Failed to create temporary directory '{}' : {}",build_path.display(),err)
            ).into_log())?;

        let stat_path = info_path.join("score.toml");
        let old_stats = util::parse_from::<StatBlockSet>(&stat_path)?;

        let mut new_stats : StatBlockSet = Default::default();

        for member in context.members.clone().iter() {
            match Self::latest_score(&old_stats,member,build_path.path(),&spec,context) {
                Ok(Some(block)) => if new_stats.stat_block.is_some() {
                    new_stats.stat_block.as_mut().unwrap().push(block);
                } else {
                    new_stats.stat_block = Some(vec![block]);
                }
                Err(log) => print!("{}",log),
                _ => {
                    println!("{}",format!(
                        "{} has no submission.",
                        member.clone().into_string().unwrap()
                    ).yellow().bold());
                },
            }
        }

        util::serialize_into(&stat_path, &new_stats)?;

        Ok(())
    }

    fn update_all_scores(context : &mut Context) -> Result<(),FailLog>
    {
        let ok_asgn : Vec<OsString> = context.manifest.iter()
            .filter_map(|name| context.catalog.get(name) )
            .filter_map(|asgn| asgn.as_ref().ok() )
            .map(|asgn| OsString::from(asgn.name.clone()))
            .collect();

        let mut log = FailLog::new();
        for asgn in ok_asgn.iter() {
            if let Err(err) = Self::update_scores(asgn.clone(),context) {
                log.extend(err);
            }
        }
        if ! log.empty() {
            print!("{}",log);
        }
        Ok(())
    }


    pub fn execute(self, context: &mut Context) -> Result<(),FailLog>
    {
        use InstructorAct::*;
        match self {
            Grader(act)                => act.execute(context),
            //Audit    { asgn_name }   => Self::audit(asgn_name.clone(),context),
            //AuditAll {}              => Self::audit_all(context),
            ListAsgns   {}             => context.list_asgns(),
            ListSubs    {asgn,user}    => context.list_subs(
                asgn.as_ref().unwrap_or(&None).as_ref(),
                user.as_ref().unwrap_or(&None).as_ref(),
            ),
            AddStudents {user_names}   => Self::add_students(user_names,context),
            RemStudents {user_names}   => Self::remove_students(user_names,context),
            AddGraders  {user_names}   => Self::add_graders(user_names,context),
            RemGraders  {user_names}   => Self::remove_graders(user_names,context),
            AddAsgns    {asgn_names}   => Self::add_assignments(asgn_names,context),
            RemAsgns    {asgn_names}   => Self::remove_assignments(asgn_names,context),
            SetDue      {asgn,date}    => Self::set_due(asgn,date,context),
            SetOpen     {asgn,date}    => Self::set_open(asgn,date,context),
            SetClose    {asgn,date}    => Self::set_close(asgn,date,context),
            UnsetDue    {asgn}         => Self::unset_due(asgn,context),
            UnsetOpen   {asgn}         => Self::unset_open(asgn,context),
            UnsetClose  {asgn}         => Self::unset_close(asgn,context),
            Publish     {asgn}         => Self::publish(asgn,context),
            Unpublish   {asgn}         => Self::unpublish(asgn,context),
            Enable      {asgn}         => Self::enable(asgn,context),
            Disable     {asgn}         => Self::disable(asgn,context),
            UpdateScores{asgn}         => Self::update_scores(asgn,context),
            UpdateAllScores{}          => Self::update_all_scores(context),
            Extend   {asgn,user,ext}   => Self::extend(asgn,user,ext,context),
            SetGrace {asgn,user,ext}   => StudentAct::grace(&asgn,&user,ext,context),
            GraceTotal  {num}          => Self::grace_total(num,context),
            GraceLimit  {num}          => Self::grace_limit(num,context),
            Refresh     {}             => context.refresh(),
        }
    }

}

