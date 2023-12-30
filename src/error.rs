use std::{fmt, path::PathBuf};
use colored::Colorize;

#[derive(Debug, Clone)]
pub enum Error {
    NoBaseDir(PathBuf),
    LocalBuildFail(String),
    DestBuildFail(String),
    FormatFail(String),
    StyleFail(String),
    TestFail(String),
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
    Unauthorized(),
    BeforeOpen,
    AfterClose,
    Inactive,
    NoGrace,
    NotEnoughGrace,
    GraceLimit,
    Custom(String, String),
}

impl Error {
    fn description(&self) -> String {
        use Error::*;
        match self {
            NoBaseDir(dir)      => format!("{} '{}' {}", "Base submission directory for course".red(), dir.to_string_lossy(), "does not exist".red()),
            NoSpec(name, desc)  => format!( "{} {} {} {}", "Specification file for".red(), name, "could not be read, IO Error:".red(), desc),
            BadSpec(name, desc) => format!( "{} {} {} {}", "Specification file for".red(), name, "is malformed, Parse Error:".red(), desc),
            IOFail(desc)        => format!( "{} '{}'", "IO Failure".red(), desc),
            InvalidAsgn(name)   => format!("{} '{}' {}", "Assignment".red(), name, "is invalid or non-existant.".red()),
            InvalidUser(name)   => format!("{} '{}' {}", "User".red(), name, "is invalid or non-existant".red()),
            InvalidCWD()        => format!("{}", "Failed to access Current Working Directory.".red()),
            InvalidUID(uid)     => format!("{}", format!("UID {uid} is invalid.").red()),
            LocalBuildFail(err) => format!("{}\n\n{}", "Build failure in current working directory:".red(), err),
            DestBuildFail(err)  => format!("{}\n\n{}", "Build failure in submission directory:".red(), err),
            FormatFail(err)     => format!("{}\n\n{}", "Failed to format files. Error:".red(), err),
            StyleFail(err)      => format!("{}\n\n{}", "Failed to check style. Error:".red(), err),
            TestFail(err)       => format!("{}\n\n{}", "Failed to test functionality due to internal error. Error:".red(), err),
            MissingFile(name)   => format!("{} '{}' {}", "File".red(), name, "does not exist in current working directory.".red()),
            MissingSub(name)    => format!("{} '{}' {}", "File".red(), name, "does not exist in the submission directory".red()),
            FileIsDir(name)     => format!("{} '{}' {}", "File".red(), name, "is actually a directory".red()),
            FileIsOther(name)   => format!("{} '{}' {}", "File".red(), name, "in neither a file nor a directory".red()),
            NoSetup(name)       => format!("{} '{}'", "Setup files are not available for assignment".red(), name),
            Unauthorized()      => format!("{}", "Action is not authorized".red()),
            BeforeOpen          => format!("{}", "Assignments cannot be interacted with before their open date.".red()),
            AfterClose          => format!("{}", "Assignments cannot be interacted with after their close date.".red()),
            Inactive            => format!("{}", "Interaction with this assignment is currently disabled.".red()),
            NoGrace             => format!("{}", "This course does not provide grace days.".red()),
            NotEnoughGrace      => format!("{}", "There aren't enough free grace days to provide such an extension.".red()),
            GraceLimit          => format!("{}", "The number of grace days requested exceeds the per-assignment grace day limit.".red()),
            Custom(text, _)     => format!("{}", text.red()),
        }
    }

    fn advice(&self) -> String {
        use Error::*;
        match self {
            NoBaseDir(_)
            | NoSpec(_, _)
            | BadSpec(_, _)
            | IOFail(_)
            | InvalidUID(_)
            | TestFail(_)
            | Unauthorized() => format!("{}", "Please contact the instructor.".yellow()),

            InvalidAsgn(_)
            | InvalidUser(_)
            | NoSetup(_)
            | BeforeOpen
            | AfterClose
            | Inactive => format!("{}", "If this is an error, please contact the instructor.".yellow()),

            LocalBuildFail(_)
            | FormatFail(_)
            | StyleFail(_) => format!("{}", "Please fix the required errors.".yellow()),

            MissingFile(name)
            | FileIsDir(name)
            | FileIsOther(name) => format!("{} '{}' {}", "Please ensure that".yellow(), name, "is a file.".yellow()),

            InvalidCWD()     => format!("{}", "Please change to a valid directory.".yellow()),
            DestBuildFail(_) => format!("{}", "Please ensure that only the files listed by the assignment are necessary for compilation.".yellow()),
            MissingSub(name) => format!("{} '{}' {}", "File", name, "cannot be recovered.".yellow()),
            NoGrace          => format!("{}", "Assignments should be turned in on-time for full credit.".yellow()),
            NotEnoughGrace   => format!("{}", "To increase the number of available grace days, remove grace days from other assignments.".yellow()),
            GraceLimit       => format!("{}", "Assignments should be turned in before the grace day limit for full credit.".yellow()),
            Custom(_, text)  => format!("{}", text.yellow()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{}", self.description(), self.advice())
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

impl fmt::Display for ErrorLog {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.iter().try_for_each(|item|
            writeln!(
                f, "{} {}\n{} {}",
                "!".red(), item.description(), ">".yellow(), &item.advice(),
            )
        )
    }
}
