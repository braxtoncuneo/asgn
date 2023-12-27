#![allow(dead_code)]
pub mod act;
pub mod asgn_spec;
pub mod context;
pub mod fail_info;
pub mod util;
pub mod table;

use std::
{
    ffi::
    {
        OsString,
    },
    path::
    {
        PathBuf,
    }
};

use structopt::StructOpt;

use fail_info::
{
    FailInfo,
    FailLog,
};

use context::
{
    Context,
    Role,
};


use asgn_spec::AsgnSpec;
use colored::Colorize;

fn check_file_exists(base_path: &PathBuf, file_name: &OsString) -> Result<(),FailInfo>
{
    use FailInfo::*;
    let path = base_path.join(file_name);
    if        ! path.exists()  {
        Err(MissingFile(file_name.clone()))
    } else if   path.is_dir()  {
        Err(FileIsDir(file_name.clone()))
    } else if ! path.is_file() {
        Err(FileIsOther(file_name.clone()))
    } else {
        Ok(())
    }
}

fn check_files(context: &Context, spec: &AsgnSpec) -> Result<(),FailLog>
{
    let log : FailLog = spec.file_list.iter()
        .map(|x| check_file_exists(&context.cwd,&x))
        .filter(|x| x.is_err())
        .map(|x| x.unwrap_err())
        .collect();

    log.result()

}

fn attempt_submission(context: &Context, spec: &AsgnSpec) -> Result<(),FailLog>
{

    check_files(context,spec)?;
    println!("Assignment '{}' submitted successfully.",spec.name);
    Ok(())
}







fn main()
{

    //let cmd = act::student::StudentCmd::from_args();

    let mut args = std::env::args().peekable();

    args.next();

    let Some(base_path) = args.next() else {
        println!("USAGE:");
        println!("asgn <base_path> <SUBCOMMAND>");
        return;
    };

    let ctx_try = Context::deduce(OsString::from(&base_path));

    let mut context = match ctx_try {
        Ok(cont) => if args.peek() == Some(&"init".to_string()) {
            print!("{}",FailInfo::Custom(
                "Provided path is already the base path of a pre-existing, valid course directory.".to_string(),
                "Either clear out that directory, or use a different one.".to_string()
            ));
            return;
        } else {
            cont
        },
        Err(err) => if args.peek() == Some(&"init".to_string()) {
            if let Err(log) = context::init(&PathBuf::from(&base_path)) {
                print!("{}",log);
            }
            return;
        } else {
            println!("{}",err);
            return;
        },
    };

    let result = match &context.role {
        Role::Instructor => {
            //context.announce();
            let cmd = act::instructor::InstructorCmd::from_args();
            cmd.act.execute(&mut context)
        },
        Role::Grader => {
            let cmd = act::grader::GraderCmd::from_args();
            cmd.act.execute(&context)
        },
        Role::Student => {
            let cmd = act::student::StudentCmd::from_args();
            cmd.act.execute(&context)
        },
        Role::Other => {
            //let cmd = act::other::OtherCmd::from_args();
            //cmd.act.execute(&context)
            println!("{}","! User not recognized as member of course.".red());
            println!("{}","> If you believe you are a member, contact the instructor.".yellow());
            return;
        }
    };


    if let Err(log) = result {
        print!("{}",log);
    }

    if let Role::Instructor = &context.role {
        if args.peek() != Some(&"refresh".to_string()) {
            context.print_failures();
        }
    }

    /*
    let opt = ::from_args();
    println!("{:?}",opt);

    let asgn_name = &args[1];

    let context = match SubmissionContext::new(&OsString::from(asgn_name)) {
        Ok(context)    => context,
        Err(fail_info) => {
            print!("{}",fail_info);
            return
        },
    };
    context.announce();

    let check = (context.asgn_name == "check") && (context.user == OsString::from("bcuneo"));

    let manifest = match AsgnSpec::generate_manifest(check) {
        Ok(manifest)   => manifest,
        Err(fail_info) => {
            print!("{}",fail_info);
            return
        },
    };

    let asgn_spec = get_spec(&manifest,&context.asgn_name);

    let asgn_spec = match asgn_spec {
        Some(spec) => spec,
        None => return,
    };


    for spec in manifest.iter() {
        println!("! {}",spec.name);
    }

    if let Err(fail_log) = attempt_submission(&context,&asgn_spec) {
        println!("{}",fail_log);
    }
    */

}



