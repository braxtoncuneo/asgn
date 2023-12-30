use structopt::StructOpt;
use colored::Colorize;
use super::other::OtherAct;
use util::bashrc_append_line;

use std::{fs, path::{Path, PathBuf}, str::FromStr, fmt};

use crate::{
    asgn_spec::{AsgnSpec, Ruleset, StatBlockSet, SubmissionFatal},
    context::{Context, Role},
    error::{Error, ErrorLog},
    util,
    table::Table,
};

#[derive(Debug, StructOpt)]
#[structopt(
    name       = "asgn - student version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct StudentCmd {
    #[structopt(name = "base path")]
    _base_path: PathBuf, // Used only to consume the first CLI arg

    #[structopt(subcommand)]
    pub act: StudentAct,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "snake")]
pub enum StudentAct {
    #[structopt(flatten)]
    Other(OtherAct),

    // Everyone
    #[structopt(about = "submits assignments (or tells you why they cannot be submitted)")]
    Submit {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "copies setup code for assignments (if provided by the instructor)")]
    Setup {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "recovers the last submitted version of the input assignment (or tells you why they cannot be recovered)")]
    Recover {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "summarizes information about submissions and currently visible assignments")]
    Summary {},
    #[structopt(about = "gives details about a specific assignment")]
    Details {
        #[structopt(name = "assignment name")]
        asgn_name: String,
    },

    #[structopt(about = "\"installs\" an alias to your .bashrc")]
    Alias {
        #[structopt(name = "alias name")]
        alias_name: String,
    },

    #[structopt(about = "assigns an integer number of grace days to an assignment")]
    Grace {
        #[structopt(name = "assignment name")]
        asgn: String,
        #[structopt(name = "grace amount")]
        ext: i64,
    },

    #[structopt(about = "lists the scores for an assignment, ordered by the given score in ascending order")]
    RankAscending {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "score name")]
        score: String,
    },

    #[structopt(about = "lists the scores for an assignment, ordered by the given score in descending order")]
    RankDescending {
        #[structopt(name = "assignment name")]
        asgn_name: String,
        #[structopt(name = "score name")]
        score: String,
    },
}

#[allow(dead_code)]
impl StudentAct {
    fn copy_dir(dst_dir: impl AsRef<Path>, src_dir: impl AsRef<Path>) -> Result<(), Error> {
        fs::create_dir_all(&dst_dir).map_err(|err|
            Error::IOFail(err.to_string())
        )?;

        let dir_iter = fs::read_dir(src_dir).map_err(|err|
            Error::IOFail(err.to_string())
        )?;

        for entry in dir_iter {
            let entry = entry.map_err(|err|
                Error::IOFail(err.to_string())
            )?;
            let ty = entry.file_type().map_err(|err|
                Error::IOFail(err.to_string())
            )?;

            if ty.is_dir() {
                StudentAct::copy_dir(dst_dir.as_ref().join(entry.file_name()), entry.path())?;
            } else {
                fs::copy(entry.path(), dst_dir.as_ref()
                    .join(entry.file_name()))
                    .map_err(|err| Error::IOFail(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub fn verify_active(spec: &AsgnSpec, context: &Context) -> Result<(), Error> {
        let is_instructor : bool = context.role != Role::Instructor;

        if !spec.active {
            return Err(Error::Inactive);
        }

        if !is_instructor && spec.before_open() {
            return Err(Error::BeforeOpen);
        }

        if !is_instructor && spec.after_close() {
            return Err(Error::AfterClose);
        }

        Ok(())
    }

    pub fn grace(asgn_name: &str, username: &str, ext_days: i64, context: &Context) -> Result<(), Error> {
        if context.grace_total.is_none() {
            return Err(Error::NoGrace);
        } else if let Some(num) = context.grace_limit.as_ref() {
            if *num < ext_days {
                return Err(Error::GraceLimit);
            }
        }

        let spec = context.catalog_get(asgn_name)?;

        Self::verify_active(spec, context)?;

        let slot = context.get_slot(spec, username);
        let current_grace = slot.get_grace()?;

        if let Some(num) = context.grace_total.as_ref() {
            if *num < (context.grace_spent() - current_grace + ext_days){
                return Err(Error::NotEnoughGrace);
            }
        }

        slot.set_grace(ext_days)
    }

    fn read_score<T>(asgn : &AsgnSpec, student_name: &str, score_name: &str) -> Result<T, Error>
    where
        T: FromStr,
        <T as FromStr>::Err: fmt::Display,
    {
        let path = asgn.path.join(".info").join("ranking").join(student_name).join(score_name);
        let text = fs::read_to_string(path).map_err(|err|
            Error::IOFail(err.to_string())
        )?;

        T::from_str(&text).map_err(|err|
            Error::IOFail(format!("Failed to parse score {score_name} for student {student_name}: {err}"))
        )
    }

    fn rank_specialized<T>(
        asgn: &AsgnSpec,
        ruleset: &Ruleset,
        rule_name: &str,
        up: bool,
        context: &Context,
    ) -> Result<(), Error>
    where
        T: FromStr + PartialOrd,
        <T as FromStr>::Err: fmt::Display,
    {
        let score_names : Vec<String> = ruleset.rules.iter().map(|r|r.target.clone()).collect();
        let mut header: Vec<String> = vec!["User".to_owned()];
        header.extend(score_names.iter().cloned());
        let mut table: Table = Table::new(header.len());
        table.add_row(header.clone())?;

        let mut rows: Vec<(Option<T>, Vec<Option<String>>)> = Vec::new();

        let base_path = asgn.path.join(".info").join("score.toml");
        let scores = util::parse_from::<StatBlockSet>(&base_path)?;

        for member in &context.members {
            let member_name = member.clone();
            let mut row = vec![Some(member.clone())];
            let stat_block = scores.get_block(&member_name);

            let Some(stat_block) = stat_block else {
                row.resize_with(header.len(), || None);
                continue;
            };

            let score: Option<T> = stat_block.scores.get(rule_name)
                .map(|toml_val|
                    T::from_str(&toml_val.to_string()).map_err(|err|
                        Error::IOFail(format!("Failed to parse score {rule_name} for user {member_name}: {err}"))
                    )
                )
                .transpose()?;

            for rule in &ruleset.rules {
                row.push(stat_block.scores.get(&rule.target).map(toml::Value::to_string));
            }

            rows.push((score, row));
        }

        rows.sort_by(|(a, _), (b, _)| {
            match (a, b) {
                (Some(a_score), Some(b_score)) => {
                    let ord = a_score.partial_cmp(b_score).unwrap();
                    if up { ord } else { ord.reverse() }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None)    => std::cmp::Ordering::Equal,
            }
        });

        for (_, row) in rows.into_iter() {
            let row_text: Vec<_> = row.into_iter()
                .map(|entry| entry.unwrap_or_else(|| "None".to_owned()))
                .collect();

            table.add_row(row_text)?;
        }

        print!("{table}");

        Ok(())
    }

    fn rank(asgn_name: &str, rule_name: &str, up: bool, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;

        let Some(ruleset) = spec.score.as_ref() else {
            return Err(Error::Custom(
                format!("Assignment '{}' has no scores to rank.", asgn_name),
                "If you believe this assignment should have scores, contact the instructor.".to_owned()
            ));
        };

        let kind = ruleset.rules.iter()
            .find(|rule| rule.target == rule_name)
            .map(|r| r.kind.clone())
            .ok_or(Error::Custom(
                format!("Assignment '{asgn_name}' does not have a '{rule_name}' score."),
                "If you believe this assignment should have this score, contact the instructor.".to_owned()
            ))?;


        let Some(kind) = kind else {
            return Err(Error::IOFail("No score kind given.".to_owned()));
        };

        match kind.as_str() {
            "bool"  => Self::rank_specialized::<bool>(spec, ruleset, rule_name, up, context),
            "int"   => Self::rank_specialized::<i64 >(spec, ruleset, rule_name, up, context),
            "float" => Self::rank_specialized::<f64 >(spec, ruleset, rule_name, up, context),
            _       => Err(Error::IOFail("Invalid score kind.".to_owned()))
        }
    }

    fn submit(asgn_name: &str, context: &Context) -> Result<(), ErrorLog> {
        let spec = context.catalog_get(asgn_name)?;

        Self::verify_active(spec, context)?;

        let sub_dir = context.base_path.join(asgn_name).join(&context.username);

        let src_dir = context.cwd.clone();
        let mut log = ErrorLog::default();
        for file_name in &spec.file_list {
            let src_path = src_dir.join(file_name);
            let dst_path = sub_dir.join(file_name);

            if !src_path.exists() {
                log.push(Error::MissingFile(file_name.clone()));
                continue;
            }
            if src_path.is_dir() {
                log.push(Error::FileIsDir(file_name.clone()));
                continue;
            }
            if !src_path.is_file() {
                log.push(Error::FileIsOther(file_name.clone()));
                continue;
            }
            fs::copy(&src_path, &dst_path).map_err(|err|
                Error::IOFail(format!(
                    "could not copy file {} to {}: {}",
                    src_path.display(), dst_path.display(), err
                ))
            )?;
            util::set_mode(&dst_path, 0o777)?;
        }
        log.into_result()?;

        util::print_bold_hline();
        println!("{}", format!("Assignment '{asgn_name}' submitted!").green());

        let build_result = spec.run_on_submit(
            context,
            spec.build.as_ref(),
            &sub_dir,
            "Building",
            false,
        );
        if build_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }

        let check_result = spec.run_on_submit(
            context,
            spec.check.as_ref(),
            &sub_dir,
            "Evaluating Checks",
            false,
        );
        if check_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }

        let score_result = spec.run_on_submit(
            context,
            spec.score.as_ref(),
            &sub_dir,
            "Evaluating Scores",
            false,
        );
        if score_result == Some(Err(SubmissionFatal)) {
            return Ok(());
        }

        util::print_bold_hline();
        Ok(())
    }

    fn setup(asgn_name: &str, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;

        Self::verify_active(spec, context)?;

        let setup_dir = context.base_path
            .join(asgn_name)
            .join(".info")
            .join("setup");

        if !setup_dir.exists() {
            return Err(Error::NoSetup(asgn_name.to_owned()));
        }

        let dst_dir = util::make_fresh_dir(&context.cwd, &format!("{asgn_name}_setup"));

        StudentAct::copy_dir(dst_dir, setup_dir)
    }

    fn recover(asgn_name: &str, context: &Context) -> Result<(), ErrorLog> {
        let spec = context.catalog_get(asgn_name)?;

        Self::verify_active(spec, context)?;

        let sub_dir = context.base_path.join(asgn_name).join(&context.username);
        let dst_dir = util::make_fresh_dir(&context.cwd, &format!("{asgn_name}_recovery"));

        fs::create_dir_all(&dst_dir).map_err(|err|
            Error::IOFail(err.to_string())
        )?;

        let mut log: ErrorLog = Default::default();
        for file_name in &spec.file_list {
            let src_path = sub_dir.join(file_name);
            if !src_path.exists() {
                log.push(Error::MissingSub(file_name.clone()));
                continue;
            }
            let dst_path = dst_dir.join(file_name);

            fs::copy(&src_path, &dst_path).map_err(|err|
                Error::IOFail(format!(
                    "could not copy file {} to {}: {}",
                    src_path.display(), dst_path.display(), err)
                )
            )?;
        }
        log.into_result()
    }

    fn alias(alias_name: &str, context: &Context) -> Result<(), Error> {
        let line = format!(
            "alias {}=\"{} {}\"",
            alias_name,
            context.exe_path.display(),
            context.base_path.display(),
        );
        bashrc_append_line(&line)?;
        println!("{}", "Alias installed successfully.".yellow());
        println!("{}", "The alias will take effect automatically for future shell sessions.".yellow());
        println!("{}", "\nTo have it take effect for this shell session, run this command:".yellow());
        println!("{}", "\n\nsource ~/.bashrc\n\n".green());

        Ok(())
    }

    fn details(asgn_name: &str, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;

        if ! spec.visible {
            return Err(Error::InvalidAsgn(asgn_name.to_owned()));
        }

        print!("{}", spec.details(context)?);
        Ok(())
    }

    pub fn execute(&self, context: &Context) -> Result<(), ErrorLog> {
        use StudentAct::*;
        match self {
            Other          ( act        ) => act.execute(context)?,
            Submit         { asgn_name  } => Self::submit (asgn_name, context)?,
            Setup          { asgn_name  } => Self::setup  (asgn_name, context)?,
            Recover        { asgn_name  } => Self::recover(asgn_name, context)?,
            Summary        {            } => context.summary()?,
            Details        { asgn_name  } => Self::details(asgn_name, context)?,
            Grace          { asgn, ext  } => Self::grace(asgn, &context.username, *ext, context)?,
            Alias          { alias_name } => Self::alias(alias_name, context)?,
            RankAscending  { asgn_name: asgn, score} => Self::rank(asgn, score, true, context)?,
            RankDescending { asgn_name: asgn, score} => Self::rank(asgn, score, false, context)?,
        }

        Ok(())
    }
}
