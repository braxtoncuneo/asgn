use structopt::StructOpt;
use super::student::StudentAct;

use std::
{
    ffi::OsString,
    fs
};

use crate::
{
    context::Context,
    fail_info::
    {
        FailInfo,
        FailLog,
    },
    asgn_spec::AsgnSpec,
};

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

    #[structopt(name = "instructor")]
    instructor : OsString,

    #[structopt(name = "course")]
    course : OsString,

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
    /*
    #[structopt(about = "[graders only] runs checks for a specific submission")]
    Check
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
        #[structopt(name = "student user name")]
        stud_name: OsString,
    },
    #[structopt(about = "[graders only] runs checks for all submissions of a specific assignment")]
    CheckAll
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "[graders only] runs checks for assignment in current working directory")]
    CheckLocal
    {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    */
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

    fn check(asgn_name: &OsString, user_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        Ok(())
    }

    fn check_all(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        Ok(())
    }


    fn check_local(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        Ok(())
    }


    fn copy(asgn_name: &OsString, user_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())
            ?.as_ref().map_err(|err| err.clone() )?;
        let sub_path = context.base_path.join(asgn_name).join(user_name);

        let dst_dir = context.cwd.join(user_name);
        if dst_dir.is_dir() {
            fs::remove_dir_all(&dst_dir)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not remove directory {} : {}",dst_dir.display(),err)).into_log()
                })?;
        }
        if dst_dir.is_file() {
            fs::remove_file(&dst_dir)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not remove file {} : {}",dst_dir.display(),err)).into_log()
                })?;
        }
        fs::create_dir(&dst_dir);
        
        for file_name in spec.file_list.iter() {
            let src_path = sub_path.join(file_name);
            let dst_path = dst_dir .join(file_name);
            if src_path.is_dir() {
                continue;
            }
            fs::copy(src_path,dst_path);
        }
        Ok(())
    }

    fn copy_all(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let mut log : FailLog = Default::default();
        for student_name in context.students.iter() {
            Self::copy(asgn_name,student_name,context)
                .map_err(|err| log.extend(err));
        }
        log.result()
    }

    pub fn execute(&self, context: &Context) -> Result<(),FailLog>
    {
        use GraderAct::*;
        match self {
            Student(act)                  => act.execute(context),
            Copy { asgn_name, stud_name } => Self::copy(asgn_name,stud_name,context),
            CopyAll { asgn_name }         => Self::copy_all(asgn_name,context),
        }
    }

}


