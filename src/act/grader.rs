use structopt::StructOpt;
use super::student::StudentAct;

use std::path::{Path, PathBuf};

use crate::{
    asgn_spec::{AsgnSpec, SubmissionFatal},
    context::Context,
    error::{ErrorLog, Error},
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
    _base_path: PathBuf, // Used only to consume the first CLI arg

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
    fn build(asgn_name: &str, context: &Context) -> Result<(), Error> {
        let spec: &AsgnSpec = context.catalog_get(asgn_name)?;
        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context, spec.build.as_ref(), &cwd, false);

        Ok(())
    }

    fn grade(asgn_name: &str, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;
        let cwd = context.cwd.clone();

        let check_result = spec.run_on_grade(context, spec.check.as_ref(), &cwd, "Evaluating Checks", true);

        if check_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }

        let score_result = spec.run_on_grade(context, spec.score.as_ref(), &cwd, "Evaluating Scores", true);

        if score_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }

        println!("{}", util::Hline::Bold);
        println!("{}", "Evaluating Grades".yellow().bold());
        let _ = spec.run_ruleset(context, spec.grade.as_ref(), &cwd, true);

        println!("{}", util::Hline::Bold);

        Ok(())
    }

    fn check(asgn_name: &str, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;

        println!("{}", util::Hline::Bold);
        println!("{}", "Evaluating Checks".yellow().bold());
        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context, spec.check.as_ref(), &cwd, true);
        println!("{}", util::Hline::Bold);

        Ok(())
    }

    fn score(asgn_name: &str, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;

        println!("{}", util::Hline::Bold);
        println!("{}", "Evaluating Scores".yellow().bold());
        let cwd = context.cwd.clone();
        let _ = spec.run_ruleset(context, spec.score.as_ref(), &cwd, true);
        println!("{}", util::Hline::Bold);

        Ok(())
    }

    pub fn copy(asgn_name: &str, username: &str, dst_dir: Option<&Path>, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;
        let dst_dir = dst_dir.unwrap_or(&context.cwd);
        let dst_dir = util::make_fresh_dir(dst_dir, username);

        spec.retrieve_sub(&dst_dir, username)?;

        let build_result = spec.run_on_submit(context, spec.build.as_ref(), &dst_dir, "Building", false);
        if build_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }

        let check_result = spec.run_on_submit(context, spec.check.as_ref(), &dst_dir, "Evaluating Checks", true);
        if check_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }

        let score_result = spec.run_on_submit(context, spec.score.as_ref(), &dst_dir, "Evaluating Scores", true);
        if score_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }
        println!("{}", util::Hline::Bold);

        Ok(())
    }

    pub fn copy_all(asgn_name: &str, dst_dir: Option<&Path>, context: &Context) -> Result<(), Error> {
        let dst_dir = dst_dir.map(Path::to_path_buf).unwrap_or(
            util::make_fresh_dir(&context.cwd, asgn_name)
        );
        util::refresh_dir(&dst_dir, 0o700, Vec::new().iter())?;
        for member_name in &context.members {
            println!("{}", format!("Retrieving Submission for '{member_name}'").bold());
            if let Err(err) = Self::copy(asgn_name, member_name, Some(&dst_dir), context) {
                println!("{}", util::Hline::Bold);
                print!("{err}");
                println!("{}", util::Hline::Bold);
            }
        }
        Ok(())
    }

    pub fn execute(&self, context: &Context) -> Result<(), ErrorLog> {
        use GraderAct::*;
        match self {
            Student(act)                  => act.execute(context)?,
            Copy { asgn_name, stud_name } => Self::copy(asgn_name, stud_name, None, context)?,
            CopyAll { asgn_name }         => Self::copy_all(asgn_name, None, context)?,
            Build { asgn_name }           => Self::build(asgn_name, context)?,
            Grade { asgn_name }           => Self::grade(asgn_name, context)?,
            Check { asgn_name }           => Self::check(asgn_name, context)?,
            Score { asgn_name }           => Self::score(asgn_name, context)?,
        }

        Ok(())
    }
}
