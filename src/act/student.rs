use structopt::StructOpt;
use super::other::OtherAct;
use util::bashrc_append_line;

use std::{fs, path::{Path, PathBuf}, str::FromStr, fmt};

use crate::{
    asgn_spec::{AsgnSpec, Ruleset, StatBlockSet, SubmissionFatal},
    context::{Context, Role},
    error::{Error, ErrorLog, InactiveKind, FilePresenceErrorKind, CONTACT_INSTRUCTOR},
    util::{self, color::{FG_GREEN, STYLE_RESET, FG_YELLOW}},
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

impl StudentAct {
    fn copy_dir(dst_dir: impl AsRef<Path>, src_dir: impl AsRef<Path>) -> Result<(), Error> {
        fs::create_dir_all(dst_dir.as_ref()).map_err(|err|
            Error::io("Failed to create dir", &dst_dir, err)
        )?;

        let dir_iter = fs::read_dir(&src_dir).map_err(|err|
            Error::io("Failed to read dir", &src_dir, err)
        )?;

        for entry in dir_iter {
            let entry = entry.map_err(|err|
                Error::io("Failed to read dir entry", &src_dir, err)
            )?;
            let ty = entry.file_type().map_err(|err|
                Error::io("Failed to get filetype", entry.path(), err)
            )?;

            if ty.is_dir() {
                StudentAct::copy_dir(
                    dst_dir.as_ref().join(entry.file_name()),
                    entry.path(),
                )?;
            } else {
                fs::copy(
                    entry.path(),
                    dst_dir.as_ref().join(entry.file_name()),
                ).map_err(|err|
                    Error::io("Failed to copy file", entry.path(), err)
                )?;
            }
        }

        Ok(())
    }

    pub fn verify_active(spec: &AsgnSpec, context: &Context) -> Result<(), InactiveKind> {
        let is_instructor : bool = context.role == Role::Instructor;

        if !spec.active {
            return Err(InactiveKind::Inactive);
        }

        if !is_instructor && spec.before_open() {
            return Err(InactiveKind::BeforeOpen);
        }

        if !is_instructor && spec.after_close() {
            return Err(InactiveKind::AfterClose);
        }

        Ok(())
    }

    pub fn grace(asgn_name: &str, username: &str, ext_days: i64, context: &Context) -> Result<(), Error> {
        if context.grace_total.is_none() {
            return Err(Error::grace_not_in_course());
        }

        if let Some(num) = &context.grace_limit {
            if *num < ext_days {
                return Err(Error::grace_limit());
            }
        }

        let spec = context.catalog_get(asgn_name)?;

        Self::verify_active(spec, context)?;

        let slot = context.get_slot(spec, username);
        let current_grace = slot.get_grace()?;

        if let Some(num) = context.grace_total.as_ref() {
            if *num < (context.grace_spent() - current_grace + ext_days) {
                return Err(Error::grace_insufficient());
            }
        }

        slot.set_grace(ext_days)
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
        let row_width = header.len();
        let mut table: Table = Table::new(header);

        let mut rows: Vec<(Option<T>, Vec<String>)> = Vec::new();

        let scores: StatBlockSet = util::parse_toml_file(asgn.path.join(".info").join("score.toml"))?;

        for member in &context.members {
            let mut row = vec![member.clone()];
            let stat_block = scores.get_block(member);

            let Some(stat_block) = stat_block else {
                row.resize_with(row_width, || Table::NONE_REPR.to_owned());
                continue;
            };

            let score: Option<T> = stat_block.scores.get(rule_name)
                .map(|toml_val|
                    T::from_str(&toml_val.to_string()).map_err(|err|
                        Error::custom(
                            format!("Failed to parse score {rule_name} for user {member}: {err}"),
                            CONTACT_INSTRUCTOR,
                        )
                    )
                )
                .transpose()?;

            row.extend(ruleset.rules.iter()
                .map(|rule| stat_block.scores.get(&rule.target))
                .map(Table::option_repr)
            );

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

        table.extend(rows.into_iter().map(|(_, row)| row))?;

        print!("{table}");

        Ok(())
    }

    fn rank(asgn_name: &str, rule_name: &str, up: bool, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;

        let Some(ruleset) = spec.score.as_ref() else {
            return Err(Error::custom(
                format!("Assignment '{asgn_name}' has no scores to rank."),
                "If you believe this assignment should have scores, contact the instructor."
            ));
        };

        let kind = ruleset.rules.iter()
            .find(|rule| rule.target == rule_name)
            .map(|r| r.kind.clone())
            .ok_or(Error::custom(
                format!("Assignment '{asgn_name}' does not have a '{rule_name}' score."),
                "If you believe this assignment should have this score, contact the instructor."
            ))?;


        let Some(kind) = kind else {
            return Err(Error::custom("No score kind given.", "Please provide a score kind."));
        };

        match kind.as_str() {
            "bool"  => Self::rank_specialized::<bool>(spec, ruleset, rule_name, up, context),
            "int"   => Self::rank_specialized::<i64 >(spec, ruleset, rule_name, up, context),
            "float" => Self::rank_specialized::<f64 >(spec, ruleset, rule_name, up, context),
            _       => Err(Error::custom("Invalid score kind.", "Please provide a score kind."))
        }
    }

    fn submit(asgn_name: &str, context: &Context) -> Result<(), ErrorLog> {
        let spec = context.catalog_get(asgn_name)?;

        Self::verify_active(spec, context).map_err(Error::from)?;

        let sub_dir = context.base_path.join(asgn_name).join(&context.username);

        let src_dir = context.cwd.clone();
        let mut log = ErrorLog::default();
        for file_name in &spec.file_list {
            let src_path = src_dir.join(file_name);
            let dst_path = sub_dir.join(file_name);

            if let Err(err) = FilePresenceErrorKind::assert_file(&src_path) {
                log.push(err.at(src_path));
                continue;
            }

            fs::copy(&src_path, &dst_path).map_err(|err|
                Error::io("Failed to copy file", src_path, err)
            )?;
            util::set_mode(&dst_path, 0o777)?;
        }
        log.into_result()?;

        println!("{}", util::Hline::Bold);
        println!("{FG_GREEN}Assignment '{asgn_name}' submitted!{STYLE_RESET}");

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

        println!("{}", util::Hline::Bold);
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
            return Err(Error::no_setup(asgn_name));
        }

        let dst_dir = util::make_fresh_dir(&context.cwd, &format!("{asgn_name}_setup"));

        StudentAct::copy_dir(dst_dir, setup_dir)
    }

    fn recover(asgn_name: &str, context: &Context) -> Result<(), ErrorLog> {
        let spec = context.catalog_get(asgn_name)?;

        Self::verify_active(spec, context).map_err(Error::from)?;

        let sub_dir = context.base_path.join(asgn_name).join(&context.username);
        let dst_dir = util::make_fresh_dir(&context.cwd, &format!("{asgn_name}_recovery"));

        fs::create_dir_all(&dst_dir).map_err(|err|
            Error::io("Failed to create dir", &dst_dir, err)
        )?;

        let mut log: ErrorLog = Default::default();
        for file_name in &spec.file_list {
            let src_path = sub_dir.join(file_name);
            if !src_path.exists() {
                log.push(FilePresenceErrorKind::NotFound.at(file_name));
                continue;
            }
            let dst_path = dst_dir.join(file_name);

            fs::copy(&src_path, &dst_path).map_err(|err|
                Error::io("Failed to copy file", src_path, err)
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
        println!(
"{FG_YELLOW}Alias installed successfully.
The alias will be present in future shell sessions.

To import it for this shell session, run the command:
  {FG_GREEN}source ~/.bashrc{STYLE_RESET}");

        Ok(())
    }

    fn details(asgn_name: &str, context: &Context) -> Result<(), Error> {
        let spec = context.catalog_get(asgn_name)?;

        if !spec.visible {
            return Err(Error::invalid_asgn(asgn_name));
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
