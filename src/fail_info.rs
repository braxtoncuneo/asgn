
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



#[derive(Debug,Clone)]
pub enum FailInfo
{
    NoBaseDir(PathBuf),
    //NoSubDir(OsString),
    //NoSrcDir(PathBuf),
    NoSpec(String,String),
    BadSpec(String,String),
    IOFail(String),
    InvalidUID(),
    InvalidCWD(),
    InvalidAsgn(OsString),
    InvalidUser(OsString),
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
            LocalBuildFail(name)  => format!("Assignment '{}' is invalid or non-existant.",name.to_string_lossy()),
            InvalidAsgn(name)     => format!("Assignment '{}' is invalid or non-existant.",name.to_string_lossy()),
            InvalidUser(name)     => format!("User name '{}' is invalid or non-existant.",name.to_string_lossy()),
            NoSpec(name,desc)     => format!("Specification file for {} could not be read. IO Error '{}'.",name,desc),
            BadSpec(name,desc)    => format!("Specification file for {} is malformed. Parse Error '{}'.",name,desc),
            IOFail(desc)          => format!("IO Failure - {}.",desc),
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
            InvalidAsgn(name)     => format!("If you believe '{}' is a valid assignment name, please contact the instructor.",name.to_string_lossy()),
            InvalidUser(name)     => format!("If you believe '{}' is a valid user name, please contact the instructor.",name.to_string_lossy()),
            NoSpec(_name,_desc)   => format!("Please contact the instructor."),
            BadSpec(_name,_desc)  => format!("Please contact the instructor."),
            IOFail(_desc)         => format!("Please contact the instructor."),
            InvalidCWD()          => format!("Please ensure that the current working directory is valid."),
            InvalidUID()          => format!("Please contact the instructor."),
            MissingFile(name)     => format!("Please ensure '{}' is an existing file in your directory.",name.to_string_lossy()),
            FileIsDir(name)       => format!("Please ensure that '{}' is a file.",name.to_string_lossy()),
            FileIsOther(name)     => format!("Please unsure that '{}' is truely a file.",name.to_string_lossy()),
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

#[derive(Default,Clone)]
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
            acc.push_str("! ");
            acc.push_str(&item.description());
            acc.push_str("\n> ");
            acc.push_str(&item.advice());
            acc.push_str("\n");
        }
        write!(f,"{}",acc)
    }
}


