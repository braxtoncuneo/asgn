
pub mod act;
pub mod asgn_spec;
pub mod context;
pub mod fail_info;


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

use context::Context;

use act::
{
    instructor,
    grader,
    student,
    other,
};

use asgn_spec::AsgnSpec;


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

    let cmd = act::instructor::InstructorCmd::from_args();
    //let cmd = act::student::StudentCmd::from_args();

    let ctx_try = Context::deduce(OsString::from("bcuneo"),OsString::from("cpsc1430"));

    let context = match ctx_try {
        Ok(cont) => cont,
        Err(err) => {
            println!("{}",err);
            return;
        },
    };
    
    context.announce();

    let result = cmd.act.execute(&context);

    if let Err(log) = result {
        print!("{}",log);
    }

    context.print_failures();

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



