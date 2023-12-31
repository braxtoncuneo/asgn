use std::{fmt, path::PathBuf};
use crate::util::color::{FG_RED, FG_YELLOW, STYLE_RESET};

#[derive(Debug, Clone)]
pub enum Error {
    NoBaseDir(PathBuf),
    LocalBuildFail(String),
    DestBuildFail(String),
    FormatFail(String),
    StyleFail(String),
    TestFail(String),
    TableError,
    NoSpec(String, String),
    BadSpec(String, String),
    IOFail(String),
    InvalidUID(u32),
    InvalidCWD(),
    InvalidAsgn(String),
    InvalidUser(String),
    MissingFile(String),
    MissingSub(String),
    FileIsDir(String),
    FileIsOther(String),
    NoSetup(String),
    NoSuchMember(String),
    Unauthorized,
    BeforeOpen,
    AfterClose,
    Inactive,
    NoGrace,
    NotEnoughGrace,
    GraceLimit,
    Custom(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        write!(f, "{FG_RED}! ")?;
        match self {
            NoBaseDir(dir)      => write!(f, "Base submission directory for course{STYLE_RESET} '{}' {FG_RED}does not exist.", dir.display()),
            NoSpec(name, desc)  => write!(f, "Specification file for{STYLE_RESET} {name} {FG_RED}could not be read, IO Error:{STYLE_RESET} {desc}"),
            BadSpec(name, desc) => write!(f, "Specification file for{STYLE_RESET} {name} {FG_RED}is malformed:{STYLE_RESET} {desc}"),
            IOFail(desc)        => write!(f, "IO Failure:{STYLE_RESET} {desc}"),
            InvalidAsgn(name)   => write!(f, "Assignment{STYLE_RESET} '{name}' {FG_RED}is invalid or non-existant."),
            InvalidUser(name)   => write!(f, "User{STYLE_RESET} '{name}' {FG_RED}is invalid."),
            InvalidCWD()        => write!(f, "Failed to access Current Working Directory."),
            InvalidUID(uid)     => write!(f, "UID {uid} is invalid."),
            LocalBuildFail(err) => write!(f, "\n\nBuild failure in current working directory:{STYLE_RESET} {err}"),
            DestBuildFail(err)  => write!(f, "\n\nBuild failure in submission directory:{STYLE_RESET} {err}"),
            FormatFail(err)     => write!(f, "\n\nFailed to format files:{STYLE_RESET} {err}"),
            StyleFail(err)      => write!(f, "\n\nFailed to check style:{STYLE_RESET} {err}"),
            TestFail(err)       => write!(f, "\n\nFailed to test functionality due to internal error:{STYLE_RESET} {err}"),
            MissingFile(name)   => write!(f, "File{STYLE_RESET} '{name}' {FG_RED}does not exist in current working directory."),
            MissingSub(name)    => write!(f, "File{STYLE_RESET} '{name}' {FG_RED}does not exist in the submission directory."),
            FileIsDir(name)     => write!(f, "File{STYLE_RESET} '{name}' {FG_RED}is actually a directory."),
            FileIsOther(name)   => write!(f, "File{STYLE_RESET} '{name}' {FG_RED}in neither a file nor a directory."),
            NoSetup(name)       => write!(f, "Setup files are not available for assignment{STYLE_RESET} '{name}'{FG_RED}."),
            NoSuchMember(name)  => write!(f, "User{STYLE_RESET} '{name}' {FG_RED} is not a member of this course."),
            TableError          => write!(f, "Failure while constructing output table."),
            Unauthorized        => write!(f, "You are not authorized to perform this action."),
            BeforeOpen          => write!(f, "Assignments cannot be interacted with before their open date."),
            AfterClose          => write!(f, "Assignments cannot be interacted with after their close date."),
            Inactive            => write!(f, "Interaction with this assignment is currently disabled."),
            NoGrace             => write!(f, "This course does not provide grace days."),
            NotEnoughGrace      => write!(f, "There aren't enough free grace days to provide such an extension."),
            GraceLimit          => write!(f, "The number of grace days requested exceeds the per-assignment grace day limit."),
            Custom(text, _)     => f.write_str(text),
        }?;

        write!(f, "\n{FG_YELLOW}> ")?;
        match self {
            NoBaseDir(_)
            | NoSpec(_, _)
            | BadSpec(_, _)
            | IOFail(_)
            | InvalidUID(_)
            | TestFail(_)
            | TableError
            | Unauthorized => write!(f, "Please contact the instructor."),

            InvalidAsgn(_)
            | InvalidUser(_)
            | NoSetup(_)
            | NoSuchMember(_)
            | BeforeOpen
            | AfterClose
            | Inactive => write!(f, "If this is an error, please contact the instructor."),

            LocalBuildFail(_)
            | FormatFail(_)
            | StyleFail(_) => write!(f, "Please fix the required errors."),

            MissingFile(name)
            | FileIsDir(name)
            | FileIsOther(name) => write!(f, "Please ensure that{STYLE_RESET} '{name}' {FG_YELLOW}is a file."),

            InvalidCWD()     => write!(f, "Please change to a valid directory."),
            DestBuildFail(_) => write!(f, "Please ensure that only the files listed by the assignment are necessary for compilation."),
            MissingSub(name) => write!(f, "File{STYLE_RESET} '{name}' {FG_YELLOW}cannot be recovered."),
            NoGrace          => write!(f, "Assignments should be turned in on-time for full credit."),
            NotEnoughGrace   => write!(f, "To increase the number of available grace days, remove grace days from other assignments."),
            GraceLimit       => write!(f, "Assignments should be turned in before the grace day limit for full credit."),
            Custom(_, text)  => f.write_str(text)
        }?;

        write!(f, "{STYLE_RESET}")
    }
}

#[derive(Default, Clone, Debug)]
pub struct ErrorLog(Vec<Error>);

impl ErrorLog {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, info: Error) {
        self.0.push(info);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_result<T: Default>(self) -> Result<T, Self> {
        self.is_empty()
            .then(T::default)
            .ok_or(self)
    }
}

impl From<Error> for ErrorLog {
    fn from(info: Error) -> Self {
        Self(vec![info])
    }
}

impl IntoIterator for ErrorLog {
    type Item = Error;
    type IntoIter = std::vec::IntoIter<Error>;

    fn into_iter(self) -> Self::IntoIter{
        self.0.into_iter()
    }
}

impl FromIterator<Error> for ErrorLog {
    fn from_iter<I: IntoIterator<Item=Error>>(iter: I) -> Self {
        Self(Vec::<Error>::from_iter(iter))
    }
}

impl Extend<Error> for ErrorLog {
    fn extend<T: IntoIterator<Item=Error>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}
