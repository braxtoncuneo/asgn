use std::{fmt, path::{PathBuf, Path}, io};

use crate::util::color::{FG_RED, FG_YELLOW, STYLE_RESET};

pub const CONTACT_INSTRUCTOR: &str = "Please contact the instructor.";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilePresenceErrorKind {
    NotFound, FileIsDir, FileIsOther,
}

impl FilePresenceErrorKind {
    pub fn description(&self) -> &'static str {
        use FilePresenceErrorKind::*;
        match self {
            NotFound => "File not found",
            FileIsDir => "File is actually a directory",
            FileIsOther => "File is neither a normal file nor a directory",
        }
    }

    pub fn advice(&self) -> &'static str {
        use FilePresenceErrorKind::*;
        match self {
            NotFound => "Please ensure that the file exists",
            FileIsDir => "Please ensure that the file is not a directory",
            FileIsOther => "Please ensure that the file is actually a file",
        }
    }

    pub fn test(path: &Path) -> Option<Self> {
        use FilePresenceErrorKind::*;
        if !path.exists() { Some(NotFound) }
        else if path.is_dir() { Some(FileIsDir) }
        else if !path.is_file() { Some(FileIsOther) }
        else { None }
    }

    pub fn at(self, path: PathBuf) -> Error {
        Error::FilePresence(path, self)
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraceErrorKind {
    NotInCourse, Insufficient, LimitReached,
}

impl GraceErrorKind {
    pub fn description(&self) -> &'static str {
        use GraceErrorKind::*;
        match self {
            NotInCourse => "This course does not provide grace days.",
            Insufficient => "There aren't enough free grace days to provide such an extension.",
            LimitReached => "The number of grace days requested exceeds the per-assignment grace day limit.",
        }
    }

    pub fn advice(&self) -> &'static str {
        use GraceErrorKind::*;
        match self {
            NotInCourse => "Assignments should be turned in on-time for full credit.",
            Insufficient => "To increase the number of available grace days, remove grace days from other assignments.",
            LimitReached => "Assignments should be turned in before the grace day limit for full credit.",
        }
    }
}

impl From<GraceErrorKind> for Error {
    fn from(kind: GraceErrorKind) -> Self { Self::Grace(kind) }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InactiveErrorKind {
    BeforeOpen, AfterClose, Inactive
}

impl InactiveErrorKind {
    pub fn description(&self) -> &'static str {
        use InactiveErrorKind::*;
        match self {
            BeforeOpen => "Assignments cannot be interacted with before their open date.",
            AfterClose => "Assignments cannot be interacted with after their close date.",
            Inactive   => "Interaction with this assignment is currently disabled.",
        }
    }
}

impl From<InactiveErrorKind> for Error {
    fn from(kind: InactiveErrorKind) -> Self { Self::Inactive(kind) }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidCWD,
    InvalidUID(u32),
    InvalidUser(String),
    NoHomeDir,
    NoBaseDir(PathBuf),
    SpecIo(PathBuf, io::ErrorKind),
    BadSpec(PathBuf, &'static str),
    BadStats { username: String, desc: &'static str },
    Io(&'static str, PathBuf, io::ErrorKind),
    Command(String, io::ErrorKind),
    Subprocess(&'static str, String),
    InvalidDate(String),
    DateOutOfRange(chrono::DateTime<chrono::Local>),
    InvalidToml(PathBuf, toml::de::Error),
    TomlSer(&'static str, toml::ser::Error),
    InvalidAsgn { name: String },
    FilePresence(PathBuf, FilePresenceErrorKind),
    NoSetup(String),
    NoSuchMember(String),
    TableError,
    Inactive(InactiveErrorKind),
    Grace(GraceErrorKind),
    Custom(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        write!(f, "{FG_RED}! ")?;
        match self {
            InvalidCWD                  => write!(f, "Failed to access Current Working Directory."),
            InvalidUID(uid)             => write!(f, "UID {uid} is invalid."),
            InvalidUser(name)           => write!(f, "User{STYLE_RESET} '{name}' {FG_RED}is invalid."),
            NoHomeDir                   => write!(f, "Unable to determine home directory location."),
            NoBaseDir(path)             => write!(f, "Base submission directory for course{STYLE_RESET} '{}' {FG_RED}does not exist.", path.display()),
            SpecIo(path, desc)          => write!(f, "Specification file at{STYLE_RESET} {} {FG_RED}could not be read, IO Error:{STYLE_RESET} {desc}", path.display()),
            BadSpec(path, desc)         => write!(f, "Specification file at{STYLE_RESET} {} {FG_RED}is malformed:{STYLE_RESET} {desc}", path.display()),
            BadStats { username, desc } => write!(f, "Stat block for{STYLE_RESET} {username} {FG_RED}is malformed:{STYLE_RESET} {desc}"),
            Io(desc, path, kind)        => write!(f, "{desc} at {}:{STYLE_RESET} {kind}", path.display()),
            Command(cmd, kind)          => write!(f, "Failed to run command {STYLE_RESET}{cmd}{FG_RED}:{STYLE_RESET} {kind}"),
            Subprocess(desc, err)       => write!(f, "Subprocess {desc} failed:{STYLE_RESET} {err}"),
            InvalidDate(date)           => write!(f, "Invalid date:{STYLE_RESET} {date:?}"),
            DateOutOfRange(date)        => write!(f, "Date out of range:{STYLE_RESET} {date}"),
            InvalidToml(path, err)      => write!(f, "Invalid toml in {STYLE_RESET} {}{FG_RED}:{STYLE_RESET}\n{err}", path.display()),
            TomlSer(desc, err)          => write!(f, "Failed to serialize toml for {desc}:{STYLE_RESET} {err}"),
            InvalidAsgn { name }        => write!(f, "Assignment{STYLE_RESET} '{name}' {FG_RED}is invalid or non-existant."),
            FilePresence(path, kind)    => write!(f, "{}:{STYLE_RESET} {}", kind.description(), path.display()),
            NoSetup(name)               => write!(f, "Setup files are not available for assignment{STYLE_RESET} '{name}'{FG_RED}."),
            NoSuchMember(name)          => write!(f, "User{STYLE_RESET} '{name}' {FG_RED} is not a member of this course."),
            TableError                  => write!(f, "Failure while constructing output table."),
            Inactive(kind)              => f.write_str(kind.description()),
            Grace(kind)                 => f.write_str(kind.description()),
            Custom(text, _)             => f.write_str(text),
        }?;

        write!(f, "\n{FG_YELLOW}> ")?;
        match self {
            InvalidUID(_)
            | NoBaseDir(_)
            | NoHomeDir
            | SpecIo(_, _)
            | BadSpec(_, _)
            | BadStats { .. }
            | InvalidToml(_, _)
            | TomlSer(_, _)
            | Command { .. }
            | Subprocess(_, _)
            | TableError => f.write_str(CONTACT_INSTRUCTOR),

            InvalidUser(_)
            | Io(_, _, _)
            | InvalidAsgn { .. }
            | NoSetup(_)
            | NoSuchMember(_)
            | Inactive(_) => f.write_str("If you believe this is an error in the course configuration, please contact the instructor."),

            InvalidDate(_)
            | DateOutOfRange(_)      => f.write_str("Please enter a valid date."),

            FilePresence(_, kind) => f.write_str(kind.advice()),
            InvalidCWD            => f.write_str("Please change to a valid directory."),
            Grace(kind)           => f.write_str(kind.advice()),
            Custom(_, text)       => f.write_str(text)
        }?;

        write!(f, "{STYLE_RESET}")
    }
}


#[derive(Default, Clone, Debug, PartialEq)]
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
