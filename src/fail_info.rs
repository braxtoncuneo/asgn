
use std::
{
    ffi::
    {
        OsString,
    },
    fmt,
    path::
    {
        PathBuf,
    }
};

use colored::Colorize;


#[derive(Debug,Clone)]
pub enum FailInfo
{
    NoBaseDir(PathBuf),
    //NoSubDir(OsString),
    //NoSrcDir(PathBuf),
    LocalBuildFail(OsString),
    DestBuildFail(OsString),
    FormatFail(OsString),
    StyleFail(OsString),
    TestFail(OsString),
    NoSpec(String,String),
    BadSpec(String,String),
    IOFail(String),
    InvalidUID(),
    InvalidCWD(),
    InvalidAsgn(OsString),
    InvalidUser(OsString),
    MissingFile(OsString),
    MissingSub(OsString),
    FileIsDir(OsString),
    FileIsOther(OsString),
    NoSetup(OsString),
    Unauthorized(),
    BeforeOpen,
    AfterClose,
    Inactive,
    NoGrace,
    NotEnoughGrace,
    GraceLimit,
    Custom(String,String),
}

impl FailInfo
{

    fn description(&self) -> String
    {
        use FailInfo::*;
        match self {
            NoBaseDir(base_path)    => format!("{} '{}' {}",
                "Base submission directory for course".red(),
                base_path.to_string_lossy(),
                "does not exist".red()
            ),
            NoSpec(name,desc)       => format!( "{} {} {} '{}'",
                "Specification file for".red(),
                name,
                "could not be read. IO Error".red(),
                desc
            ),
            BadSpec(name,desc)      => format!( "{} {} {} '{}'",
                "Specification file for".red(),
                name,
                "is malformed. Parse Error".red(),
                desc
            ),
            IOFail(desc)            => format!( "{} '{}'",
                "IO Failure".red(),
                desc
            ),
            InvalidAsgn(name)       => format!("{} '{}' {}",
                "Assignment".red(),
                name.to_string_lossy(),
                "is invalid or non-existant.".red(),
            ),
            InvalidUser(name)       => format!("{} '{}' {}",
                "User name".red(),
                name.to_string_lossy(),
                "is invalid or non-existant".red(),
            ),
            InvalidCWD()            => format!("{}",
                "Current working directory is invalid.".red()
            ),
            InvalidUID()            => format!("{}",
                "User identifier invalid.".red()
            ),
            LocalBuildFail(err)     => format!("{}\n\n{}",
                "Build failure in current working directory:".red(),
                err.to_string_lossy()
            ),
            DestBuildFail(err)      => format!("{}\n\n{}",
                "Build failure in submission directory:".red(),
                err.to_string_lossy()
            ),
            FormatFail(err)         => format!("{}\n\n{}",
                "Failed to format files. Error:".red(),
                err.to_string_lossy()
            ),
            StyleFail(err)          => format!("{}\n\n{}",
                "Failed to check style. Error:".red(),
                err.to_string_lossy()
            ),
            TestFail(err)           => format!("{}\n\n{}",
                "Failed to test functionality due to internal error. Error:".red(),
                err.to_string_lossy()
            ),
            MissingFile(name)       => format!("{} '{}' {}",
                "File".red(),
                name.to_string_lossy(),
                "does not exist in current working directory.".red()
            ),
            MissingSub(name)        => format!("{} '{}' {}",
                "File".red(),
                name.to_string_lossy(),
                "does not exist in the submission directory".red()
            ),
            FileIsDir(name)         => format!("{} '{}' {}",
                "File".red(),
                name.to_string_lossy(),
                "is actually a directory".red(),
            ),
            FileIsOther(name)       => format!("{} '{}' {}",
                "File".red(),
                name.to_string_lossy(),
                "in neither a file nor a directory".red(),
            ),
            NoSetup(name)           => format!("{} '{}'",
                "Setup files are not available for assignment".red(),
                name.to_string_lossy()
            ),
            Unauthorized()          => format!( "{}",
                "Action is not authorized".red()
            ),
            BeforeOpen => format!("{}",
                "Assignments cannot be interacted with before their open date.".red()
            ),
            AfterClose => format!("{}",
                "Assignments cannot be interacted with after their close date.".red()
            ),
            Inactive => format!("{}",
                "Interaction with this assignment is currently disabled.".red()
            ),
            NoGrace => format!("{}",
                "This course does not provide grace days.".red()
            ),
            NotEnoughGrace => format!("{}",
                "There aren't enough free grace days to provide such an extension.".red()
            ),
            GraceLimit => format!("{}",
                "The number of grace days requested exceeds the per-assignment grace day limit.".red()
            ),
            Custom(text,_) => format!("{}",text.red()),
        }
    }

    fn advice(&self) -> String
    {
        use FailInfo::*;
        match self {
            NoBaseDir(_base_path)   => format!("{}",
                "Please contact the instructor.".yellow()
            ),
            InvalidAsgn(name)       => format!("{} '{}' {}",
                "If you believe".yellow(),
                name.to_string_lossy(),
                "is a valid assignment name, please contact the instructor.".yellow(),
            ),
            InvalidUser(name)       => format!("{} '{}' {}",
                "If you believe".yellow(),
                name.to_string_lossy(),
                "is a valid user name, please contact the instructor.".yellow(),
            ),
            NoSpec(_name,_desc)     => format!("{}",
                "Please contact the instructor.".yellow()
            ),
            BadSpec(_name,_desc)    => format!("{}",
                "Please contact the instructor.".yellow()
            ),
            IOFail(_desc)           => format!("{}",
                "Please contact the instructor.".yellow()
            ),
            InvalidCWD()            => format!("{}",
                "Please ensure that the current working directory is valid.".yellow()
            ),
            InvalidUID()            => format!("{}",
                "Please contact the instructor.".yellow()
            ),
            LocalBuildFail(_err)    => format!("{}",
                "All compilation errors must be fixed.".yellow()
            ),
            DestBuildFail(_err)     => format!("{}",
                "Please ensure that only the files listed by the assignment are necessary for compilation.".yellow()
            ),
            FormatFail(_err)        => format!("{}",
                "Please fix the errors noted above.".yellow()
            ),
            StyleFail(_err)         => format!("{}",
                "Please fix the errors noted above.".yellow()
            ),
            TestFail(_err)           => format!("{}",
                "Please contact the instructor.".yellow()
            ),
            MissingFile(name)       => format!("{} '{}' {}",
                "Please ensure".yellow(),
                name.to_string_lossy(),
                "is an existing file in your directory.".yellow(),
            ),
            MissingSub(name)        => format!("{} '{}' {}",
                "File",
                name.to_string_lossy(),
                "cannot be recovered.".yellow(),
            ),
            FileIsDir(name)         => format!("{} '{}' {}",
                "Please ensure that".yellow(),
                name.to_string_lossy(),
                "is a file.".yellow(),
            ),
            FileIsOther(name)       => format!("{} '{}' {}",
                "Please ensure that".yellow(),
                name.to_string_lossy(),
                "is truely a file.".yellow(),
            ),
            NoSetup(name)           => format!("{} '{}' {}",
                "If you believe assignment".yellow(),
                name.to_string_lossy(),
                "should have setup files, please contact the instructor".yellow(),
            ),
            Unauthorized()          => format!("{}",
                "Please contact the instructor".yellow()
            ),
            BeforeOpen => format!("{}",
                "If you believe this assignment should be open, contact the instructor.".yellow()
            ),
            AfterClose => format!("{}",
                "If you believe this assignment should not be closed yet, contact the instructor.".yellow()
            ),
            Inactive => format!("{}",
                "If you believe this assignment should be enabled, contact the instructor.".yellow()
            ),
            NoGrace => format!("{}",
                "Assignments should be turned in on-time for full credit.".yellow()
            ),
            NotEnoughGrace => format!("{}",
                "To increase the number of available grace days, remove grace days from other assignments.".yellow()
            ),
            GraceLimit => format!("{}",
                "Assignments should be turned in before the grace day limit for full credit.".yellow()
            ),
            Custom(_,text) => format!("{}",text.yellow()),
        }
    }

    pub fn into_log(self) -> FailLog
    {
        self.into()
    }

}



impl fmt::Display for FailInfo
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f,"{}\n{}",self.description(),self.advice())
    }
}

#[derive(Default,Clone,Debug)]
pub struct FailLog(Vec<FailInfo>);

impl FailLog
{

    pub fn new() -> Self
    {
        Self(Vec::new())
    }

    pub fn push(&mut self, info: FailInfo)
    {
        self.0.push(info);
    }

    pub fn empty(&self) -> bool
    {
        self.0.len() == 0
    }

    pub fn result<T: Default>(&self) -> Result<T,Self>
    {
        if self.empty() {
            Ok(Default::default())
        } else {
            Err(self.clone())
        }
    }

}

impl From<FailInfo> for FailLog
{
    fn from(info: FailInfo) -> Self
    {
        Self(vec![info])
    }
}


impl IntoIterator for FailLog
{

    type Item     = FailInfo;
    type IntoIter = std::vec::IntoIter<FailInfo>;

    fn into_iter(self) -> Self::IntoIter{
        self.0.into_iter()
    }
}

impl FromIterator<FailInfo> for FailLog
{
    fn from_iter<I: IntoIterator<Item=FailInfo>>(iter: I) -> Self {
        Self(Vec::<FailInfo>::from_iter(iter))
    }
}

impl Extend<FailInfo> for FailLog
{
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
            acc.push_str(&format!("{}", "! ".red()));
            acc.push_str(&format!("{}",&item.description()));
            acc.push_str(&format!("{}", "\n> ".yellow()));
            acc.push_str(&format!("{}",&item.advice()));
            acc.push_str("\n");
        }
        write!(f,"{}",acc)
    }
}


