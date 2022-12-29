use std::
{
    env::current_dir,
    ffi::
    {
        OsString,
        OsStr,
    },
    fmt,
    fs::{
        read_dir,
        read_to_string,
    },
    path::
    {
        PathBuf,
        Path,
    }
};

use chrono::
{
    DateTime,
    Local,
    TimeZone,
};

use users::
{
    get_user_by_uid,
    get_current_uid,
};

use serde_derive::Deserialize;
use toml;
use structopt::StructOpt;


#[derive(Debug,Clone)]
enum FailInfo
{
    NoBaseDir(PathBuf),
    //NoSubDir(OsString),
    //NoSrcDir(PathBuf),
    NoInfo(String),
    BadInfo(String),
    InvalidUID(),
    InvalidCWD(),
    MissingFile(OsString),
    FileIsDir(OsString),
    FileIsOther(OsString),
}

impl FailInfo
{

    fn description(&self) -> String
    {
        use FailInfo::*;
        match self {
            NoBaseDir(base_path)  => format!("Base submission directory for course '{}' does not exist.",base_path.to_string_lossy()),
            //NoSubDir(asgn_name)   => format!("Submission directory for assignment '{}' does not exist.",asgn_name.to_string_lossy()),
            //NoSrcDir(path)        => format!("Source directory '{}' is invalid.",path.to_string_lossy()),
            NoInfo(desc)          => format!("Assignment specification file could not be read. IO Error '{}'.",desc),
            BadInfo(desc)         => format!("Assignment specification file is malformed. Parse Error '{}'.",desc),
            InvalidCWD()          => format!("Current working directory is invalid."),
            InvalidUID()          => format!("User identifier invalid."),
            MissingFile(name)     => format!("File '{}' does not exist in current working directory.",name.to_string_lossy()),
            FileIsDir(name)       => format!("File '{}' is actually a directory.",name.to_string_lossy()),
            FileIsOther(name)     => format!("File '{}' in neither a file nor a directory.",name.to_string_lossy()),
        }
    }

    fn advice(&self) -> String
    {
        use FailInfo::*;
        match self {
            NoBaseDir(_base_path) => format!("Please contact the instructor."),
            //NoSubDir(_asgn_name)  => format!("Please contact the instructor."),
            //NoSrcDir(path)        => format!("Please ensure the directory '{}' actually exists.",path.to_string_lossy()),
            NoInfo(_desc)         => format!("Please contact the instructor."),
            BadInfo(_desc)        => format!("Please contact the instructor."),
            InvalidCWD()          => format!("Please ensure that the current working directory is valid."),
            InvalidUID()          => format!("Please contact the instructor."),
            MissingFile(name)     => format!("Please ensure '{}' is an existing file in your directory.",name.to_string_lossy()),
            FileIsDir(name)       => format!("Please ensure that '{}' is a file.",name.to_string_lossy()),
            FileIsOther(name)     => format!("Please unsure that '{}' is truely a file.",name.to_string_lossy()),
        }
    }

}



impl fmt::Display for FailInfo
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f,"{}\n{}",self.description(),self.advice())
    }
}


struct FailLog(Vec<FailInfo>);

impl FailLog
{
    fn push(&mut self, info: FailInfo)
    {
        self.0.push(info);
    }

    fn empty(&self) -> bool
    {
        self.0.len() == 0
    }

    fn result<T: Default>(self) -> Result<T,Self>
    {
        if self.empty() {
            Ok(Default::default())
        } else {
            Err(self)
        }
    }

}


impl FromIterator<FailInfo> for FailLog {
    fn from_iter<I: IntoIterator<Item=FailInfo>>(iter: I) -> Self {
        Self(Vec::<FailInfo>::from_iter(iter))
    }
}

impl Extend<FailInfo> for FailLog {
    fn extend<T: IntoIterator<Item=FailInfo>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

impl fmt::Display for FailLog
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let mut acc = String::new();
        for item in self.0.iter() {
            acc.push_str("! ");
            acc.push_str(&item.description());
            acc.push_str("\n> ");
            acc.push_str(&item.advice());
            acc.push_str("\n");
        }
        write!(f,"{}",acc)
    }
}





#[derive(Deserialize)]
struct AsgnSpecToml
{
    name      : String,
    active    : bool,
    visible   : bool,
    deadline  : toml::value::Datetime,
    file_list : toml::value::Array,
}



struct AsgnSpec
{
    name      : String,
    active    : bool,
    visible   : bool,
    deadline  : DateTime<Local>,
    file_list : Vec<OsString>,
}


impl TryFrom<AsgnSpecToml> for AsgnSpec
{
    type Error = FailInfo;

    fn try_from(spec_toml: AsgnSpecToml) -> Result<Self,Self::Error> {
        let deadline = match spec_toml.deadline.date {
            Some(date) => {
                let y : i32 = date.year  as i32;
                let m : u32 = date.month as u32;
                let d : u32 = date.day   as u32;
                chrono::offset::Local.with_ymd_and_hms(y,m,d,23,59,59).unwrap()
            },
            None       => {
                return Err(FailInfo::BadInfo(String::from("Date data missing from deadline field.")))
            },
        };
        let mut file_list = Vec::<OsString>::new();
        for entry in spec_toml.file_list.iter()
        {
            let filename = match entry.as_str() {
                Some(filename) => OsString::from(filename),
                None  => {
                    return Err(FailInfo::BadInfo(String::from("File list contains non-string entries.")))
                },
            };
            file_list.push(filename);
        }

        Ok(Self {
            name     : spec_toml.name,
            active   : spec_toml.active,
            visible  : spec_toml.visible,
            deadline,
            file_list,
        })

    }
}


impl AsgnSpec
{

    fn new(path : PathBuf) -> Result<Self,FailInfo>
    {

        let spec_path = path.join(".spec");
        let info_path = spec_path.join("info.toml");

        //println!("{}",info_path.display());

        let info_text = match read_to_string(info_path) {
            Ok(text) => text,
            Err(err) => { return Err(FailInfo::NoInfo(format!("{}",err))) },
        };

        let info_toml : AsgnSpecToml = match toml::from_str(&info_text) {
            Ok(info) => info,
            Err(err) => { return Err(FailInfo::BadInfo(format!("{}",err))) },
        };

        let result : Result<Self,FailInfo> = info_toml.try_into();

        match result.as_ref() {
            Ok(spec) => {
                if ! path.ends_with(&spec.name) {
                    return Err(FailInfo::BadInfo(String::from("Name field does not match assignment directory name.")))
                }
            },
            _ => {}
        };

        return result;

    }

    fn generate_manifest(check : bool) -> Result<Vec<AsgnSpec>,FailInfo>
    {
        let base_path : PathBuf = Path::new("/home/fac/bcuneo/submit/cpsc1430/").to_path_buf();

        if let Ok(entry_iter) = read_dir(&base_path) {
            let result_iter = entry_iter
                .filter (|x| x.is_ok() )
                .map    (|x| x.unwrap())
                .filter (|x| x.file_type().map(|y| y.is_dir()).unwrap_or(false) )
                .map    (|x| AsgnSpec::new(x.path()));
            
            let spec_list : Vec<AsgnSpec> = result_iter
                .filter (|x| { 
                    if let Err(err) = x {
                        if check { println!("{}",err) }
                    };
                    x.is_ok()
                })
                .map    (|x| x.unwrap())
                .collect();
            Ok(spec_list)
            
        } else {
            Err(FailInfo::NoBaseDir(base_path))
        }
    }

}



struct SubmissionContext
{
    user       : OsString,
    asgn_name  : OsString,
    source     : PathBuf,
    time       : DateTime<Local>,
}


impl SubmissionContext
{



    fn new (asgn_name : &OsString) -> Result<Self,FailInfo>
    {

        let Some(user) = get_user_by_uid(get_current_uid()) else {
            return Err(FailInfo::InvalidUID())
        };

        let Ok(source_dir) = current_dir() else {
            return Err(FailInfo::InvalidCWD())
        };

        Ok(Self {
            asgn_name : asgn_name.into(),
            user      : user.name().into(),
            source    : source_dir,
            time      : Local::now(),
        })
    }

    fn announce(&self)
    {
        println!("The time is : {}",self.time);
        println!("The user is : {}",self.user.to_string_lossy());
        println!("Submitting {} from directory {}",self.asgn_name.to_string_lossy(),self.source.display());       
    }

}



fn get_spec<'a> (manifest : &'a Vec<AsgnSpec>, name: &OsStr) -> Option<&'a AsgnSpec>
{
    for entry in manifest.iter() {
        if OsString::from(&entry.name) == name {
            return Some(entry);
        }
    }
    None
}



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

fn check_files(context: &SubmissionContext, spec: &AsgnSpec) -> Result<(),FailLog>
{
    let log : FailLog = spec.file_list.iter()
        .map(|x| check_file_exists(&context.source,&x))
        .filter(|x| x.is_err())
        .map(|x| x.unwrap_err())
        .collect();

    log.result()
    
}

fn source_build(_context: &SubmissionContext, _spec: &AsgnSpec) -> Result<(),FailLog>
{
    Ok(())
}

fn destination_build(_context: &SubmissionContext, _spec: &AsgnSpec) -> Result<(),FailLog>
{
    Ok(())
}


fn attempt_submission(context: &SubmissionContext, spec: &AsgnSpec) -> Result<(),FailLog>
{

    check_files(context,spec)?;
    source_build(context,spec)?;
    destination_build(context,spec)?;
    println!("Assignment '{}' submitted successfully.",context.asgn_name.to_string_lossy());
    Ok(())
}



enum Role
{
    Instructor,
    Grader,
    Student,
}



impl Role
{


}



#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn - student version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
enum StudentOpt
{

    // Everyone
    #[structopt(about = "submits assignments")]
    Submit{
        asgn_name: OsString,
    },
    #[structopt(about = "copies setup code for assignments (if provided by the instructor)")]
    Setup{
        asgn_name: OsString,
    },
    #[structopt(about = "lists assignments currently published by the instructor")]
    List{},
}





#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn - grader version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
enum GraderOpt
{
    #[structopt(flatten)]
    Student(StudentOpt),

    // Instructors and Graders
    #[structopt(about = "[graders only] runs checks for a specific submission")]
    Check
    {
        asgn_name: OsString,
        stud_name: OsString,
    },
    #[structopt(about = "[graders only] runs checks for all submissions of a specific assignment")]
    CheckAll
    {
        asgn_name: OsString,
    },
}


#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn - instructor version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
enum InstructorOpt
{
    #[structopt(flatten)]
    Grader(GraderOpt),

    // Instructors Only
    #[structopt(about = "[instructors only] sets the base submission directory")]
    SetBase
    {
        #[structopt(parse(from_os_str))]
        base_path: PathBuf,
    },
    #[structopt(about = "[instructors only] checks an assignment specification for validity")]
    Audit
    {
        asgn_name: OsString,
    },
    #[structopt(about = "[instructors only] checks all assignment specifications for validity")]
    AuditAll{},

}






/*
impl Action
{

    fn parse(args: &Vec<String> ) -> Self {
        use Action::*;

        if args.len() == 0 {
            return Help();
        }

        match (args[0],args.len()) => {
            ("set_base",    2)  => SetBase(OsString::from(args[1])),
            ("audit",       2)  => Audit(OsString::from(args[1])),
            ("audit_all",   1)  => AuditAll(),
            ("check",       3)  => Check(OsString::from(args[1]),Opstring::from(args[2])),
            ("check_all",   2)  => CheckAll(OsString::from(args[1])),
            ("submit",      2)  => Submit(OsString::from(args[1])),
            ("setup",       2)  => Setup(OsString::from(args[1])),
            ("list",        1)  => List(),
            ("help",        1)  => Help(),
        }
    }

    fn restriction(&self) -> ModeRestriction
    {
        match self {
            SetBase(_) => Instructors,
            Audit(_)   => Instructors,
            AuditAll() => Instructors,
            Check(_)   => Graders,
            CheckAll() => Graders,
            Submit(_)  => Everyone,
            Setup(_)   => Everyone,
            List()     => Everyone,
            Help()     => Everyone,
        }
    }

}
*/


fn main()
{
    let opt = StudentOpt::from_args();
    println!("{:?}",opt);

    let args : Vec<_> = std::env::args().collect();
    if args.len() <= 1 {
        println!("No argument supplied. To submit, an assignment name should be provided.");
        return;
    } else if args.len() > 2 {
        println!("Too many arguments supplied.");
        return;
    }

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

}



