use structopt::StructOpt;
use std::{
    ffi::OsString,
    str::FromStr,
};


use crate::
{
    context::Context,
    fail_info::
    {
        FailLog,
        FailInfo,
    },
    asgn_spec::AsgnSpec,
    act::{
        student::StudentAct,
        grader::GraderAct,
    },
    util,
};

use colored::Colorize;

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
    #[structopt(about = "[instructors only] adds the listed students to the course")]
    AddStudents
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] removes the listed students from the course")]
    RemStudents
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] adds the listed graders to the course")]
    AddGraders
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] removes the listed graders from the course")]
    RemGraders
    {
        #[structopt(name = "user names")]
        user_names: Vec<OsString>,
    },
    #[structopt(about = "[instructors only] lists all assignments, including unpublished ones")]
    FullSummary {},
    #[structopt(about = "[instructors only] summarizes information about all submissions")]
    AllSubs {},
    #[structopt(about = "[instructors only] summarizes information about a student's submissions")]
    StudentSubs
    {
        #[structopt(name = "student user name")]
        user : OsString,
    },
    #[structopt(about = "[instructors only] summarizes information about an assignment's submissions")]
    AssignSubs {
        #[structopt(name = "assignment name")]
        asgn: OsString,
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
    #[structopt(about = "[instructors only] updates published ranks for a given assignment based upon current submissions")]
    UpdateRanks{
        #[structopt(name = "assignment name")]
        asgn: OsString,
    },
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
        context.sync()
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
        Self::add_students(user_names,context)
    }

    fn remove_graders(user_names: Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        context.graders.retain(|os_str| ! user_names.contains(os_str) );
        Self::remove_students(user_names,context)
    }

    fn get_mut_spec<'a>(asgn: &OsString, context: &'a mut Context) -> Result<&'a mut AsgnSpec,FailLog> {
        context.catalog.get_mut(asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone())
    }

    fn set_due(asgn: OsString, date: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        let date = toml::value::Datetime::from_str(&date.to_string_lossy())
            .map_err(|err| FailInfo::IOFail(format!("{}",err)))?;
        spec.due_date = Some(AsgnSpec::date_into_chrono(date)?);
        spec.sync()
    }

    fn set_open(asgn: OsString, date: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        let date = toml::value::Datetime::from_str(&date.to_string_lossy())
            .map_err(|err| FailInfo::IOFail(format!("{}",err)))?;
        spec.open_date = Some(AsgnSpec::date_into_chrono(date)?);
        spec.sync()
    }

    fn set_close(asgn: OsString, date: OsString, context: &mut Context) -> Result<(),FailLog>
    {
        let spec : &mut AsgnSpec = Self::get_mut_spec(&asgn,context)?;
        let date = toml::value::Datetime::from_str(&date.to_string_lossy())
            .map_err(|err| FailInfo::IOFail(format!("{}",err)))?;
        spec.close_date = Some(AsgnSpec::date_into_chrono(date)?);
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

    fn publish(asgn: OsString, context: &mut Context) -> Result<(),FailLog> {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.visible = true;
        spec.sync()
    }

    fn unpublish(asgn: OsString, context: &mut Context) -> Result<(),FailLog> {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.visible = false;
        spec.sync()
    }

    fn enable(asgn: OsString, context: &mut Context) -> Result<(),FailLog> {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.active = true;
        spec.sync()
    }

    fn disable(asgn: OsString, context : &mut Context) -> Result<(),FailLog> {
        let spec : &mut AsgnSpec = context.catalog.get_mut(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_mut().map_err(|err| err.clone() )?;
        spec.active = false;
        spec.sync()
    }


    fn extend(asgn: OsString, user: OsString, ext_days: i64, context : &Context) -> Result<(),FailLog> {
        let spec : &AsgnSpec = context.catalog.get(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        let slot = context.get_slot(spec,&user);
        slot.set_extension(ext_days)
    }


    fn update_ranks(asgn: OsString, context : &mut Context) -> Result<(),FailLog> {
        let spec : &AsgnSpec = context.catalog.get(&asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        let info_path  = spec.path.join(".info");
        let score_path = info_path.join("ranking");
        let build_path = info_path.join("score_builds");
        if build_path.exists() {
            std::fs::remove_dir_all(&build_path)
                .map_err(|err|FailInfo::IOFail(
                    format!("Failed to remove directory '{}' : {}",build_path.display(),err)
                ).into_log())?;
        }
        GraderAct::copy_all(&asgn,Some(&build_path),context)?;
        for member in context.members.iter() {
            let member_build_dir = build_path.join(member);
            let member_score_dir = score_path.join(member);
            if let Some(score) = &spec.score {
                if ! score.on_submit.unwrap_or(true) {
                    util::print_bold_hline();
                    println!("{}",format!("Scoring Submission for '{}'",member.to_string_lossy()).bold());
                    let _ = spec.run_ruleset(context,spec.score.as_ref(),&member_build_dir);
                    util::print_bold_hline();
                }
                for rule in score.rules.iter() {
                    let member_score = member_build_dir.join(&rule.target);
                    let public_score  = member_score_dir.join(&rule.target);
                    if ! member_score.exists() {
                        continue;
                    }
                    println!("{} -> {}",member_score.display(),public_score.display());
                    std::fs::copy(&member_score,&public_score)
                        .map_err(|err|FailInfo::IOFail(format!(
                            "Failed to copy '{}' to '{}' : {}",
                            &member_score.display(),&public_score.display(),err
                        )).into_log())?;
                }
            }
        }
        util::recursive_refresh_dir(build_path,0o700,Vec::new().iter())?;
        Ok(())
    }



    pub fn execute(self, context: &mut Context) -> Result<(),FailLog>
    {
        use InstructorAct::*;
        match self {
            Grader(act)              => act.execute(context),
            //Audit    { asgn_name } => Self::audit(asgn_name.clone(),context),
            //AuditAll {}            => Self::audit_all(context),
            FullSummary {}           => context.summary(true,None,Some(&context.user)),
            AllSubs     {}           => context.summary(true,None,None),
            StudentSubs {user}       => context.summary(true,None,Some(&user)),
            AssignSubs  {asgn}       => context.summary(true,Some(&asgn),None),
            AddStudents {user_names} => Self::add_students(user_names,context),
            RemStudents {user_names} => Self::remove_students(user_names,context),
            AddGraders  {user_names} => Self::add_graders(user_names,context),
            RemGraders  {user_names} => Self::remove_graders(user_names,context),
            SetDue      {asgn,date}  => Self::set_due(asgn,date,context),
            SetOpen     {asgn,date}  => Self::set_open(asgn,date,context),
            SetClose    {asgn,date}  => Self::set_close(asgn,date,context),
            UnsetOpen   {asgn}       => Self::unset_open(asgn,context),
            UnsetClose  {asgn}       => Self::unset_close(asgn,context),
            Publish     {asgn}       => Self::publish(asgn,context),
            Unpublish   {asgn}       => Self::unpublish(asgn,context),
            Enable      {asgn}       => Self::enable(asgn,context),
            Disable     {asgn}       => Self::disable(asgn,context),
            UpdateRanks {asgn}       => Self::update_ranks(asgn,context),
            Extend   {asgn,user,ext} => Self::extend(asgn,user,ext,context),
            SetGrace {asgn,user,ext} => StudentAct::grace(&asgn,&user,ext,context),
            GraceTotal  {num}        => Self::grace_total(num,context),
            GraceLimit  {num}        => Self::grace_limit(num,context),
            Refresh     {}           => context.refresh(),
        }
    }

}

