use structopt::StructOpt;
use super::grader::GraderAct;
use std::ffi::OsString;

use crate::
{
    context::Context,
    fail_info::
    {
        FailLog,
    },
};



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
    
    #[structopt(about = "[instructors only] attempts to fix the state of the course directory")]
    Refresh{},

}



impl InstructorAct
{

    fn add_students(user_names: &Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        for name in user_names.iter() {
            if ! context.students.contains(&mut name.clone()) {
                context.students.push(name.clone());
            }
        }
        context.sync_course_spec()
    }

    fn remove_students(user_names: &Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        context.students.retain(|os_str| ! user_names.contains(os_str) );
        context.sync_course_spec()
    }

    fn add_graders(user_names: &Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        for name in user_names.iter() {
            if ! context.graders.contains(&mut name.clone()) {
                context.graders.push(name.clone());
            }
        }
        Self::add_students(user_names,context)
    }

    fn remove_graders(user_names: &Vec<OsString>, context: &mut Context) -> Result<(),FailLog>
    {
        context.graders.retain(|os_str| ! user_names.contains(os_str) );
        Self::remove_students(user_names,context)
    }

    fn audit(_asgn_name: OsString, _context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }


    fn audit_all(_context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }



    pub fn execute(&self, context: &mut Context) -> Result<(),FailLog>
    {
        use InstructorAct::*;
        match self {
            Grader(act)            => act.execute(context),
            //Audit    { asgn_name } => Self::audit(asgn_name.clone(),context),
            //AuditAll {}            => Self::audit_all(context),
            AddStudents {user_names} => Self::add_students(user_names,context),
            RemStudents {user_names} => Self::remove_students(user_names,context),
            AddGraders  {user_names} => Self::add_graders(user_names,context),
            RemGraders  {user_names} => Self::remove_graders(user_names,context),
            Refresh  {}            => context.refresh(),
        }
    }

}

