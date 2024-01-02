use std::{fmt, path::Path, io};

use crate::util::color::{FG_RED, FG_YELLOW, STYLE_RESET};

pub const CONTACT_INSTRUCTOR: &str = "Please contact the instructor.";
pub const MAYBE_CONTACT_INSTRUCTOR: &str =
    "If you believe this is an error in the course configuration, contact the instructor.";
pub const VALID_DATE: &str = "Please enter a valid date";

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

    pub fn assert_file(path: &Path) -> Result<(), Self> {
        use FilePresenceErrorKind::*;
        if !path.exists() { Err(NotFound) }
        else if path.is_dir() { Err(FileIsDir) }
        else if !path.is_file() { Err(FileIsOther) }
        else { Ok(()) }
    }

    pub fn at(self, path: impl AsRef<Path>) -> Error {
        Error::file_presence(path, self)
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InactiveKind {
    BeforeOpen, AfterClose, Inactive
}

impl InactiveKind {
    pub fn description(&self) -> &'static str {
        use InactiveKind::*;
        match self {
            BeforeOpen => "Assignments cannot be interacted with before their open date.",
            AfterClose => "Assignments cannot be interacted with after their close date.",
            Inactive   => "Interaction with this assignment is currently disabled.",
        }
    }
}

impl From<InactiveKind> for Error {
    fn from(kind: InactiveKind) -> Self { Self::inactive(kind) }
}


#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    description: String,
    advice: String
}

impl Error {
    fn new(description: impl ToString, advice: impl ToString) -> Self {
        Self {
            description: description.to_string(),
            advice: advice.to_string(),
        }
    }

    pub fn invalid_cwd(err: io::Error) -> Self {
        Self::new(
            format!("Failed to access Current Working Directory: {err}"),
            "Please change to a valid directory.",
        )
    }

    pub fn invalid_uid(uid: u32) -> Self {
        Self::new(
            format!("UID {uid} is invalid."),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn invalid_user(username: &str) -> Self {
        Self::new(
            format!("User{STYLE_RESET} '{username}' {FG_RED}is invalid."),
            MAYBE_CONTACT_INSTRUCTOR,
        )
    }

    pub fn no_home_dir() -> Self {
        Self::new(
            "Unable to determine home directory location.",
            MAYBE_CONTACT_INSTRUCTOR,
        )
    }

    pub fn no_base_dir(path: impl AsRef<Path>) -> Self {
        Self::new(
            format!("Base submission directory for course{STYLE_RESET} '{}' {FG_RED}does not exist.", path.as_ref().display()),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn spec_io(path: impl AsRef<Path>, err: io::Error) -> Self {
        Self::new(
            format!("Specification file at{STYLE_RESET} {} {FG_RED}could not be read, IO Error:{STYLE_RESET} {err}", path.as_ref().display()),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn bad_spec(path: impl AsRef<Path>, desc: &str) -> Self {
        Self::new(
            format!("Specification file at{STYLE_RESET} {} {FG_RED}is malformed:{STYLE_RESET} {desc}", path.as_ref().display()),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn bad_stats(username: &str, desc: &str) -> Self {
        Self::new(
            format!("Stat block for{STYLE_RESET} {username} {FG_RED}is malformed:{STYLE_RESET} {desc}"),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn io(desc: &str, path: impl AsRef<Path>, err: io::Error) -> Self {
        Self::new(
            format!("{desc} at {}:{STYLE_RESET} {err}", path.as_ref().display()),
            MAYBE_CONTACT_INSTRUCTOR,
        )
    }

    pub fn command(cmd: &str, err: io::Error) -> Self {
        Self::new(
            format!("Failed to run command {STYLE_RESET}{cmd}{FG_RED}:{STYLE_RESET} {err}"),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn subprocess(desc: &str, err: String) -> Self {
        Self::new(
            format!("Subprocess {desc} failed:{STYLE_RESET} {err}"),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn invalid_date(date: &str) -> Self {
        Self::new(
            format!("Invalid date:{STYLE_RESET} {date:?}"),
            VALID_DATE,
        )
    }

    pub fn date_out_of_range(date: chrono::DateTime<chrono::Local>) -> Self {
        Self::new(
            format!("Date out of range:{STYLE_RESET} {date}"),
            VALID_DATE,
        )
    }

    pub fn invalid_toml(path: impl AsRef<Path>, err: toml::de::Error) -> Self {
        Self::new(
            format!("Invalid toml in {STYLE_RESET} {}{FG_RED}:{STYLE_RESET}\n{err}", path.as_ref().display()),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn toml_ser(desc: &'static str, err: toml::ser::Error) -> Self {
        Self::new(
            format!("Failed to serialize toml for {desc}:{STYLE_RESET} {err}"),
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn invalid_asgn(name: &str) -> Self {
        Self::new(
            format!("Assignment{STYLE_RESET} '{name}' {FG_RED}is invalid or non-existant."),
            MAYBE_CONTACT_INSTRUCTOR,
        )
    }

    pub fn file_presence(path: impl AsRef<Path>, kind: FilePresenceErrorKind) -> Self {
        Self::new(
            format!("{}:{STYLE_RESET} {}", kind.description(), path.as_ref().display()),
            kind.advice(),
        )
    }

    pub fn no_setup(name: &str) -> Self {
        Self::new(
            format!("Setup files are not available for assignment{STYLE_RESET} '{name}'{FG_RED}."),
            MAYBE_CONTACT_INSTRUCTOR,
        )
    }

    pub fn no_such_member(name: &str) -> Self {
        Self::new(
            format!("User{STYLE_RESET} '{name}' {FG_RED} is not a member of this course."),
            MAYBE_CONTACT_INSTRUCTOR,
        )
    }

    pub fn table_error() -> Self {
        Self::new(
            "Failure while constructing output table.",
            CONTACT_INSTRUCTOR,
        )
    }

    pub fn inactive(kind: InactiveKind) -> Self {
        Self::new(
            kind.description(),
            MAYBE_CONTACT_INSTRUCTOR,
        )
    }

    pub fn grace_not_in_course() -> Self {
        Self::new(
            "This course does not provide grace days.",
            "Assignments should be turned in on-time for full credit.",
        )
    }

    pub fn grace_insufficient() -> Self {
        Self::new(
            "There aren't enough free grace days to provide such an extension.",
            "To increase the number of available grace days, remove grace days from other assignments.",
        )
    }

    pub fn grace_limit() -> Self {
        Self::new(
            "The number of grace days requested exceeds the per-assignment grace day limit.",
            "Assignments should be turned in before the grace day limit for full credit.",
        )
    }

    pub fn custom(description: impl ToString, advice: impl ToString) -> Self {
        Self::new(description, advice)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{FG_RED}! {}\n{FG_YELLOW}> {}{STYLE_RESET}", self.description, self.advice)
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

    pub fn into_result<T: Default>(self) -> Result<T, Self> {
        self.0.is_empty()
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
