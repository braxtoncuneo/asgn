use structopt::StructOpt;
use super::student::StudentAct;

use std::path::{Path, PathBuf};

use crate::{
    asgn_spec::AsgnSpec,
    context::Context,
    fail_info::{FailInfo, FailLog},
    util,
};

use colored::Colorize;

#[derive(Debug, StructOpt)]
#[structopt(
    name       = "asgn - grader version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]

pub struct GraderCmd {
    #[structopt(name = "base path")]
    base_path: PathBuf,

    #[structopt(subcommand)]
    pub act: GraderAct,
}


#[derive(Debug, StructOpt)]
#[structopt(rename_all = "snake")]
pub enum GraderAct {
    #[structopt(flatten)]
    Student(StudentAct),

    // Instructors and Graders
    #[structopt(about = "[graders only] runs build rules for an assignment, using cwd as the submission location")]
    Build {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },
    #[structopt(about = "[graders only] run grade rules for an assignment, using cwd as the submission location")]
    Grade {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },
    #[structopt(about = "[graders only] runs check rules for an assignment, using cwd as the submission location")]
    Check {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },
    #[structopt(about = "[graders only] runs score rules for an assignment, using cwd as the submission location")]
    Score {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },
    #[structopt(about = "[graders only] copies the directory of a submission to cwd")]
    Copy {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "student name")]
        stud_name: String,
    },
    #[structopt(about = "[graders only] copies the directory of all submissions of an assignment to cwd")]
    CopyAll {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },
}

impl GraderAct {
    fn build(asgn_name: &str, context: &Context) -> Result<(), FailLog> {
        let spec: &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.to_owned()).into_log())?
            .as_ref().map_err(Clone::clone)?;

        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context,spec.build.as_ref(),&cwd,false).is_err();

        Ok(())
    }

    fn grade(asgn_name: &str, context: &Context) -> Result<(), FailLog> {
        let spec: &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.to_owned()).into_log())?
            .as_ref().map_err(|err| err.clone())?;
        let cwd = context.cwd.clone();

        let check_result  = spec.run_on_grade(context,spec.check.as_ref(),&cwd,"Evaluating Checks",true);

        if check_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }

        let score_result = spec.run_on_grade(context,spec.score.as_ref(),&cwd,"Evaluating Scores",true);

        if score_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }

        util::print_bold_hline();
        println!("{}","Evaluating Grades".yellow().bold());
        let _ = spec.run_ruleset(context,spec.grade.as_ref(),&cwd,true);

        util::print_bold_hline();

        Ok(())
    }

    fn check(asgn_name: &str, context: &Context) -> Result<(),FailLog> {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.to_owned()).into_log())?
            .as_ref().map_err(|err| err.clone())?;

        util::print_bold_hline();
        println!("{}","Evaluating Checks".yellow().bold());
        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context,spec.check.as_ref(),&cwd,true).is_err();
        util::print_bold_hline();

        Ok(())
    }

    fn score(asgn_name: &str, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.to_owned()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        util::print_bold_hline();
        println!("{}","Evaluating Scores".yellow().bold());
        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context,spec.score.as_ref(),&cwd,true).is_err();
        util::print_bold_hline();

        Ok(())
    }


    pub fn copy(asgn_name: &str, user_name: &str, dst_dir: Option<&Path>, context: &Context) -> Result<(),FailLog> {
        let spec: &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.to_owned()).into_log())
            ?.as_ref().map_err(|err| err.clone() )?;

        let dst_dir = dst_dir.unwrap_or(&context.cwd);

        let dst_dir = util::make_fresh_dir(dst_dir,user_name);

        spec.retrieve_sub(&dst_dir,user_name)?;

        let build_result = spec.run_on_submit(context,spec.build.as_ref(),&dst_dir,"Building",false);
        if build_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }

        let check_result = spec.run_on_submit(context,spec.check.as_ref(),&dst_dir,"Evaluating Checks",true);
        if check_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }

        let score_result = spec.run_on_submit(context,spec.score.as_ref(),&dst_dir,"Evaluating Scores",true);
        if score_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }
        util::print_bold_hline();


        Ok(())
    }

    pub fn copy_all(asgn_name: &str, dst_dir: Option<&Path>, context: &Context) -> Result<(),FailLog>
    {
        let dst_dir = dst_dir.map(|p|p.to_path_buf()).unwrap_or(
            util::make_fresh_dir(&context.cwd, asgn_name)
        );
        util::refresh_dir(&dst_dir,0o700,Vec::new().iter())?;
        for member_name in context.members.iter() {
            println!("{}",format!("Retrieving Submission for '{member_name}'").bold());
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
