use structopt::StructOpt;
use super::student::StudentAct;

use std::
{
    ffi::OsString,
    path::Path,
};

use crate::
{
    asgn_spec::AsgnSpec,
    context::Context,
    fail_info::
    {
        FailInfo,
        FailLog,
    },
    util,
};

use colored::Colorize;

#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn - grader version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct GraderCmd
{

    #[structopt(name = "base path")]
    base_path : OsString,

    #[structopt(subcommand)]
    pub act: GraderAct,
}


#[derive(Debug,StructOpt)]
#[structopt(rename_all = "snake")]
pub enum GraderAct
{
    #[structopt(flatten)]
    Student(StudentAct),

    // Instructors and Graders
    #[structopt(about = "[graders only] runs build rules for a specific assignment, using the current working directory as the location of the submission")]
    Build
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "[graders only] run grade rules for a specific assignment, using the current working directory as the location of the submission")]
    Grade
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "[graders only] runs check rules for a specific assignment, using the current working directory as the location of the submission")]
    Check
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "[graders only] runs score rules for a specific assignment, using the current working directory as the location of the submission")]
    Score
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "[graders only] copies the directory of a specific submission to the current working directory")]
    Copy
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
        #[structopt(name = "student name")]
        stud_name: OsString,
    },
    #[structopt(about = "[graders only] copies the directory of all submissions of a specific assignment to the current working directory")]
    CopyAll
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
}



impl GraderAct
{



    fn build(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context,spec.build.as_ref(),&cwd).is_err();

        Ok(())
    }

    fn grade(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        let cwd = context.cwd.clone();

        if ! spec.run_on_grade(context,spec.check.as_ref(),&cwd,"Evaluating Checks") {
            return Ok(());
        }

        if ! spec.run_on_grade(context,spec.score.as_ref(),&cwd,"Evaluating Scores") {
            return Ok(());
        }

        util::print_bold_hline();
        println!("{}","Evaluating Grades".yellow().bold());
        let _ = spec.run_ruleset(context,spec.grade.as_ref(),&cwd).is_err();

        util::print_bold_hline();

        Ok(())
    }

    fn check(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        util::print_bold_hline();
        println!("{}","Evaluating Checks".yellow().bold());
        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context,spec.check.as_ref(),&cwd).is_err();
        util::print_bold_hline();

        Ok(())
    }

    fn score(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        util::print_bold_hline();
        println!("{}","Evaluating Scores".yellow().bold());
        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context,spec.score.as_ref(),&cwd).is_err();
        util::print_bold_hline();

        Ok(())
    }


    pub fn copy(asgn_name: &OsString, user_name: &OsString, dst_dir : Option<&Path>, context: &Context)
    -> Result<(),FailLog>
    {

        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())
            ?.as_ref().map_err(|err| err.clone() )?;

        let dst_dir = dst_dir.unwrap_or(&context.cwd);

        let dst_dir = util::make_fresh_dir(dst_dir,&user_name.to_string_lossy());

        spec.retrieve_sub(&dst_dir,&user_name.to_string_lossy())?;

        if ! spec.run_on_submit(context,spec.build.as_ref(),&dst_dir,"Building") {
            return Ok(());
        }

        if ! spec.run_on_submit(context,spec.check.as_ref(),&dst_dir,"Evaluating Checks") {
            return Ok(());
        }

        if ! spec.run_on_submit(context,spec.score.as_ref(),&dst_dir,"Evaluating Scores") {
            return Ok(());
        }
        util::print_bold_hline();


        Ok(())
    }

    pub fn copy_all(asgn_name: &OsString, dst_dir : Option<&Path>, context: &Context) -> Result<(),FailLog>
    {
        let dst_dir = dst_dir.map(|p|p.to_path_buf()).unwrap_or(
            util::make_fresh_dir(&context.cwd,&asgn_name.to_string_lossy())
        );
        util::refresh_dir(&dst_dir,0o700,Vec::new().iter())?;
        for member_name in context.members.iter() {
            println!("{}",format!("Retrieving Submission for '{}'",member_name.to_string_lossy()).bold());
            if let Err(err) = Self::copy(asgn_name,member_name,Some(&dst_dir),context){
                util::print_bold_hline();
                print!("{}",err);
                util::print_bold_hline();
            }
        }
        Ok(())
    }

    pub fn execute(&self, context: &Context) -> Result<(),FailLog>
    {
        use GraderAct::*;
        match self {
            Student(act)                  => act.execute(context),
            Copy { asgn_name, stud_name } => Self::copy(asgn_name,stud_name,None,context),
            CopyAll { asgn_name }         => Self::copy_all(asgn_name,None,context),
            Build { asgn_name }           => Self::build(asgn_name,context),
            Grade { asgn_name }           => Self::grade(asgn_name,context),
            Check { asgn_name }           => Self::check(asgn_name,context),
            Score { asgn_name }           => Self::score(asgn_name,context),
        }
    }

}


