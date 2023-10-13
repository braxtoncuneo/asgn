use structopt::StructOpt;
use super::grader::GraderAct;

use crate::
{
    context::Context,
    fail_info::
    {
        FailLog,
    },
};

use std::ffi::OsString;


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

    #[structopt(name = "instructor")]
    instructor : OsString,

    #[structopt(name = "course")]
    course : OsString,

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
    /*
    #[structopt(about = "[instructors only] adds the listed students to the course")]
    AddStudents
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

    fn add_students(_user_names: &Vec<OsString>, _context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }

    fn add_graders(_user_names: &Vec<OsString>, _context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }


    fn audit(_asgn_name: OsString, _context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }


    fn audit_all(_context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }



    pub fn execute(&self, context: &Context) -> Result<(),FailLog>
    {
        use InstructorAct::*;
        match self {
            Grader(act)            => act.execute(context),
            //Audit    { asgn_name } => Self::audit(asgn_name.clone(),context),
            //AuditAll {}            => Self::audit_all(context),
            Refresh  {}            => context.refresh(),
        }
    }

}

