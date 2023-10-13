use structopt::StructOpt;
use colored::Colorize;

use std::
{
    ffi::OsString,
    fs,
    path::Path,
};

use crate::
{
    asgn_spec::
    {
        AsgnSpec,
        SubmissionSlot,
    },
    context::Context,
    fail_info::
    {
        FailInfo,
        FailLog,
    },
    util,
};


#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn - student version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct StudentCmd
{

    #[structopt(name = "instructor")]
    instructor : OsString,

    #[structopt(name = "course")]
    course : OsString,

    #[structopt(subcommand)]
    pub act: StudentAct,
}



#[derive(Debug,StructOpt)]
#[structopt(rename_all = "snake")]
pub enum StudentAct
{

    // Everyone
    #[structopt(about = "submits assignments (or tells you why they cannot be submitted)")]
    Submit{
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "copies setup code for assignments (if provided by the instructor)")]
    Setup{
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "recovers the last submitted version of the input assignment (or tells you why they cannot be recovered)")]
    Recover{
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "summarizes information about submissions and currently visible assignments")]
    Summary{},
}


impl StudentAct
{

    fn copy_dir(dst_dir: impl AsRef<Path>, src_dir : impl AsRef<Path>) -> Result<(),FailLog> {
        fs::create_dir_all(&dst_dir)
            .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;

        let dir_iter = fs::read_dir(src_dir)
            .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;

        for entry in dir_iter {
            let entry = entry
                .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;
            let ty = entry.file_type()
                .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;
            if ty.is_dir() {
                StudentAct::copy_dir(dst_dir.as_ref().join(entry.file_name()),entry.path())?;

            } else {
                fs::copy(entry.path(), dst_dir.as_ref().join(entry.file_name()))
                    .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;
            }
        }
        return Ok(())
    }


    fn check() {

    }


    fn submit(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        
        let sub_dir = context.base_path.join(asgn_name).join(&context.user);

        let src_dir = context.cwd.clone();
        let mut log : FailLog = Default::default();
        for file_name in spec.file_list.iter() {
            let src_path = src_dir.join(file_name);
            let dst_path = sub_dir.join(file_name);
            if ! src_path.exists() {
                log.push(FailInfo::MissingFile(file_name.clone()).into());
                continue;
            }
            if src_path.is_dir() {
                log.push(FailInfo::FileIsDir(file_name.clone()).into());
                continue;
            }
            if ! src_path.is_file() {
                log.push(FailInfo::FileIsOther(file_name.clone()).into());
                continue;
            }
            fs::copy(&src_path,&dst_path)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not copy file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)).into_log()
                })?;
            util::set_mode(&dst_path,0o777)?;
        }
        log.result()?;

        let (status,_out,err) = util::run_at(context.build_command(spec),&src_dir,false)?;

        if ! status.success() {
            return Err(FailInfo::LocalBuildFail(err).into())
        }
        println!("{}","[X] Submission compiles in the current working directory.".green());


        let (status,_out,err) = util::run_at(context.build_command(spec),&sub_dir,false)?;

        if ! status.success() {
            return Err(FailInfo::DestBuildFail(err).into())
        }
        println!("{}","[X] Submission compiles in the submission directory.".green());

        let (status,_out,err) = util::run_at(context.format_command(spec),&sub_dir,false)?;

        if ! status.success() {
            return Err(FailInfo::FormatFail(err).into())
        }
        println!("{}","[X] Submission format adjusted.".green());


        let (status,out,_err) = util::run_at(context.style_command(spec),&sub_dir,false)?;

        for file_name in spec.file_list.iter() {
            let src_path = src_dir.join(file_name);
            let dst_path = sub_dir.join(file_name);
            fs::copy(&dst_path,&src_path)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not copy format file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)).into_log()
                })?;
        }

        if ! status.success() {
            return Err(FailInfo::StyleFail(out).into())
        }


        if ! out.is_empty() {
            println!("{}","[~] Submission style checking resulted in non-empty output. Output below:".yellow());
            print!("{}",out.to_string_lossy());
        } else {
            println!("{}","[X] Submission style checked.".green());
        }
            
        let (_status,_out,_err) = util::run_at(context.test_command(spec),&sub_dir,true)?;

        println!("{}",format!("Assignment '{}' submitted!",asgn_name.to_string_lossy()).green());

        Ok(())
    }

    fn setup(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let _spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        
        let setup_dir = context.base_path
            .join(asgn_name).join(".spec").join("setup");

        if ! setup_dir.exists() {
            return Err(FailInfo::NoSetup(asgn_name.clone()).into());
        }

        let mut index = 0;
        while context.cwd.join(format!("setup.{}",index)).exists() {
            index += 1;
        }
        let dst_dir = context.cwd.join(format!("setup.{}",index));

        StudentAct::copy_dir(dst_dir,setup_dir)
    }

    fn recover(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {

        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        
        let sub_dir = context.base_path.join(asgn_name).join(&context.user);

        let mut index = 0;
        while context.cwd.join(format!("recovery.{}",index)).exists() {
            index += 1;
        }
        let dst_dir = context.cwd.join(format!("recovery.{}",index));
        
        fs::create_dir_all(&dst_dir)
            .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;

        let mut log : FailLog = Default::default();
        for file_name in spec.file_list.iter() {
            let src_path = sub_dir.join(file_name);
            let dst_path = dst_dir.join(file_name);
            if ! src_path.exists() {
                log.push(FailInfo::MissingSub(file_name.clone()).into());
                continue;
            }
            fs::copy(&src_path,&dst_path)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not copy file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)).into_log()
                })?;
        }
        log.result()

    }

    fn summary(context: &Context) -> Result<(),FailLog>
    {
        println!("| {:<12} | {:<11} | {:<20} | {}","NAME","DUE DATE", "SUBMISSION STATUS", "FILES");
        for asgn in context.manifest.iter()
            .filter_map(|name| context.catalog[name].as_ref().ok())
        {
            if ! asgn.visible || ! asgn.active {
                continue;
            }
        
            let sub_dir = context.base_path.join(&asgn.name).join(&context.user);

            let slot = SubmissionSlot {
                context,
                asgn_spec: asgn,
                base_path: sub_dir,
            };

            let status = slot.status();
            let lateness = status.unwrap().versus(&asgn.deadline);

            println!("| {:<12} | {:<11} | {:<20} | {}","","","","");
            let deadline = asgn.deadline.date_naive();
            //let turnin   = asgn.status();
            print!("| {:<12} | {:<11}  | ",asgn.name,deadline);
            print!("{:<20} | ",lateness);
            for file in asgn.file_list.iter() {
                print!("{}  ",file.to_string_lossy());
            }
            println!("");
        }
        Ok(())
    }

    pub fn execute(&self, context: &Context) -> Result<(),FailLog>
    {
        use StudentAct::*;
        match self {
            Submit  { asgn_name } => Self::submit (asgn_name,context),
            Setup   { asgn_name } => Self::setup  (asgn_name,context),
            Recover { asgn_name } => Self::recover(asgn_name,context),
            Summary {}            => Self::summary(context),
        }
    }

}


