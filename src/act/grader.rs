use structopt::StructOpt;
use super::student::StudentAct;

use std::
{
    ffi::OsString,
    fs,
    process::Command,
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
    util::
    {
        self,
    },
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

    fn check(_asgn_name: &OsString, _user_name: &OsString, _context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }

    fn check_all(_asgn_name: &OsString, _context: &Context) -> Result<(),FailLog>
    {
        todo!()
    }


    fn check_local(_asgn_name: &OsString, _context: &Context) -> Result<(),FailLog>
    {
        todo!()
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
        fs::create_dir(&dst_dir)
            .map_err(|err| {
                FailInfo::IOFail(format!("could not create directory {} : {}",dst_dir.display(),err)).into_log()
            })?;
        
        for file_name in spec.file_list.iter() {
            let src_path = sub_path.join(file_name);
            let dst_path = dst_dir .join(file_name);
            if src_path.is_dir() {
                continue;
            }
            fs::copy(&src_path,&dst_path)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not copy file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)).into_log()
                })?;
        }
            
        let src_format_path = context.base_path.join(".clang-format");
        let src_style_path  = context.base_path.join(".clang-tidy");
        let dst_format_path = dst_dir.join(".clang-format");
        let dst_style_path  = dst_dir.join(".clang-tidy");
        fs::copy(&src_format_path,&dst_format_path)
            .map_err(|err| {
                FailInfo::IOFail(format!("could not copy format file {} to {} : {}",
                (&src_format_path).display(),(&dst_format_path).display(),err)).into_log()
            })?;
        fs::copy(&src_style_path,&dst_style_path)
            .map_err(|err| {
                FailInfo::IOFail(format!("could not copy style file {} to {} : {}",
                (&src_style_path).display(),(&dst_style_path).display(),err)).into_log()
            })?;


        let (status,_out,err) = util::run_at(context.build_command(spec),&dst_dir,false)?;
        if ! status.success() {
            let name = asgn_name.to_string_lossy();
            let comp_name = (name.clone() + ".comp").to_string();
            let comp_path = dst_dir.join(&comp_name);
            fs::write(&comp_path,err.to_string_lossy().to_string())
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not write compiler output to {} : {}",
                    (&comp_path).display(),err)).into_log()
                })?;
            println!("Assignment files failed to build. Compiler output written to {}",comp_name);
            return Ok(())
        }
        
        let style_dir = dst_dir.join(".style");
        util::refresh_dir(&style_dir,0o700,Vec::new().iter())?;        
        for file_name in spec.file_list.iter() {
            let src_path =   dst_dir.join(file_name);
            let dst_path = style_dir.join(file_name);
            if src_path.is_dir() {
                continue;
            }
            fs::copy(&src_path,&dst_path)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not copy file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)).into_log()
                })?;
        }

        let (status,_out,err) = util::run_at(context.format_command(spec),&style_dir,false)?;
        if ! status.success() {
            return Err(FailInfo::FormatFail(err).into())
        }
        let (_status,out,_err) = util::run_at(context.style_command(spec),&style_dir,false)?;

        if ! out.is_empty() {
            let name = asgn_name.to_string_lossy();
            let warn_name = (name.clone() + ".warn").to_string();
            let warn_path = dst_dir.join(&warn_name);
            fs::write(&warn_path,out.to_string_lossy().to_string())
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not write style checker output to {} : {}",
                    (&warn_path).display(),err)).into_log()
                })?;
            println!("Submission files produced style warnings. Warnings written to {}",warn_name);
        }

        for file_name in spec.file_list.iter() {
            let cwd_path =   dst_dir.join(file_name);
            let sty_path = style_dir.join(file_name);
            let mut diff = Command::new("diff");
            diff.arg("--color=always").arg(&cwd_path).arg(&sty_path);
            let (_status,out,_err) = util::run_at(diff,&dst_dir,false)?;
            
            if ! out.is_empty() {
                let name = file_name.to_string_lossy();
                let diff_name = (name.clone() + ".diff").to_string();
                let diff_path = dst_dir.join(&diff_name);
                fs::write(&diff_path,out.to_string_lossy().to_string())
                    .map_err(|err| {
                        FailInfo::IOFail(format!("could not write format diff output to {} : {}",
                        (&diff_path).display(),err)).into_log()
                    })?;
                println!("File {name} differs from course style. Difference written to {}",diff_name);
            }
        }

        Ok(())
    }

    fn copy_all(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let mut log : FailLog = Default::default();
        for student_name in context.students.iter() {
            println!("Processing {}",student_name.to_string_lossy());
            let _ = Self::copy(asgn_name,student_name,context)
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


