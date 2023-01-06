use structopt::StructOpt;

use std::
{
    ffi::OsString,
    fs,
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
    #[structopt(about = "summarizes information about submissions and currently visible assignments")]
    Summary{},
}


impl StudentAct
{

    fn submit(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;
        
        let sub_path = context.base_path.join(asgn_name).join(&context.user);

        let src_dir = context.cwd.clone();
        let mut log : FailLog = Default::default();
        for file_name in spec.file_list.iter() {
            let src_path = src_dir .join(file_name);
            let dst_path = sub_path.join(file_name);
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
            fs::copy(src_path,dst_path);
        }
        log.result()?;

        let src_build_output = spec.build_command()
            .current_dir(src_dir)
            .output();

        if src_build_output.status != 0 {

        }

        let dst_build_output = spec.build_command()
            .current_dir(sub_dir)
            .output();

        if dst_build_output.status != 0 {

        }

        for file_name in spec.file_list.iter() {
            let file_path = sub_path.join(file_name);
            Command::
        }
        Ok(())
    }

    fn setup(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        Ok(())
    }

    fn summary(context: &Context) -> Result<(),FailLog>
    {
        println!("| {:<12} | {:<11} | {}","NAME","DUE DATE", "FILES");
        for asgn in context.manifest.iter()
            .filter_map(|name| context.catalog[name].as_ref().ok())
        {
            if ! asgn.visible {
                continue;
            }
            println!("| {:<12} | {:<11} | {}","","","");
            print!("| {:<12} | {:<11}  | ",asgn.name,asgn.deadline.date_naive());
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
            Submit  { asgn_name } => Self::submit(asgn_name,context),
            Setup   { asgn_name } => Self::setup(asgn_name,context),
            Summary {}            => Self::summary(context),
        }
    }

}


