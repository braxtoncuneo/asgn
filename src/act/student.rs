use structopt::StructOpt;
use colored::Colorize;
use super::other::OtherAct;
use util::bashrc_append_line;

use std::
{
    ffi::OsString,
    fs,
    path::Path,
    str::FromStr,
};

use crate::
{
    asgn_spec::
    {
        AsgnSpec,
        Ruleset,
        StatBlock,
        StatBlockSet,
    },
    context::{
        Context,
        Role,
    },
    fail_info::
    {
        FailInfo,
        FailLog,
    },
    util,
    table::Table,
};


#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn - student version",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct StudentCmd
{

    #[structopt(name = "base path")]
    base_path : OsString,

    #[structopt(subcommand)]
    pub act: StudentAct,
}



#[derive(Debug,StructOpt)]
#[structopt(rename_all = "snake")]
pub enum StudentAct
{
    #[structopt(flatten)]
    Other(OtherAct),
    // Everyone
    #[structopt(about = "submits assignments (or tells you why they cannot be submitted)")]
    Submit {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "copies setup code for assignments (if provided by the instructor)")]
    Setup {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "recovers the last submitted version of the input assignment (or tells you why they cannot be recovered)")]
    Recover {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "summarizes information about submissions and currently visible assignments")]
    Summary {},
    #[structopt(about = "gives details about a specific assignment")]
    Details {
        #[structopt(name = "assignment name")]
        asgn_name: OsString,
    },
    #[structopt(about = "\"installs\" an alias to your .bashrc")]
    Alias {
        #[structopt(name = "alias name")]
        alias_name: OsString,
    },
    #[structopt(about = "assigns an integer number of grace days to an assignment")]
    Grace {
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "grace amount")]
        ext : i64,
    },
    #[structopt(about = "lists the scores for an assignment, ordered by the given score in ascending order")]
    RankAscending {
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "score name")]
        score: String,
    },
    #[structopt(about = "lists the scores for an assignment, ordered by the given score in descending order")]
    RankDescending {
        #[structopt(name = "assignment name")]
        asgn: OsString,
        #[structopt(name = "score name")]
        score: String,
    },
}


impl StudentAct
{

    fn copy_dir(dst_dir: impl AsRef<Path>, src_dir : impl AsRef<Path>) -> Result<(),FailLog> {
        fs::create_dir_all(&dst_dir)
            .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;

        let dir_iter = fs::read_dir(src_dir)
            .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;

        for entry in dir_iter {
            let entry = entry
                .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;
            let ty = entry.file_type()
                .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;
            if ty.is_dir() {
                StudentAct::copy_dir(dst_dir.as_ref().join(entry.file_name()),entry.path())?;

            } else {
                fs::copy(entry.path(), dst_dir.as_ref().join(entry.file_name()))
                    .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;
            }
        }
        return Ok(())
    }


    pub fn verify_active(spec: &AsgnSpec, context: &Context) -> Result<(),FailLog> {
        let is_instructor : bool = context.role != Role::Instructor;

        if !spec.active {
            return Err(FailInfo::Inactive.into_log());
        }

        if !is_instructor && spec.before_open() {
            return Err(FailInfo::BeforeOpen.into_log());
        }

        if !is_instructor && spec.after_close() {
            return Err(FailInfo::AfterClose.into_log());
        }

        Ok(())
    }


    pub fn grace(asgn: &OsString, user: &OsString, ext_days: i64, context : &Context) -> Result<(),FailLog> {

        if context.grace_total.is_none() {
            return Err(FailInfo::NoGrace.into_log());
        } else if let Some(num) = context.grace_limit.as_ref() {
            if *num < ext_days {
                return Err(FailInfo::GraceLimit.into_log());
            }
        }

        let spec : &AsgnSpec = context.catalog.get(asgn)
            .ok_or(FailInfo::InvalidAsgn(asgn.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        Self::verify_active(spec,context)?;

        let slot = context.get_slot(spec,&user);
        let current_grace = slot.get_grace()?;

        if let Some(num) = context.grace_total.as_ref() {
            if *num < (context.grace_spent() - current_grace + ext_days){
                return Err(FailInfo::NotEnoughGrace.into_log());
            }
        }

        slot.set_grace(ext_days)
    }

    fn read_score<T: FromStr>(asgn : &AsgnSpec, student_name : &str, score_name: &str) -> Result<T,FailLog>
    {
        let path  = asgn.path.join(".info").join("ranking").join(student_name).join(score_name);
        let text  = fs::read_to_string(path)
                .map_err(|err|FailInfo::IOFail(format!("{}",err)).into_log())?;
        T::from_str(&text)
            .map_err(|_|FailInfo::IOFail(
                format!("Failed to parse score {} for student {}",score_name,student_name)
            ).into_log())
    }

    fn rank_specialized<T: FromStr + std::cmp::PartialOrd >(
        asgn: &AsgnSpec, ruleset: &Ruleset, rule_name: &str,
        up: bool, context: &Context
    )
    -> Result<(),FailLog>
    where <T as FromStr>::Err : std::fmt::Debug
    {

        let score_names : Vec<String> = ruleset.rules.iter().map(|r|r.target.clone()).collect();
        let mut header : Vec<String> = vec!["User".to_string()];
        header.extend(score_names.iter().cloned());

        let mut table : Table = Table::new(header.len());
        table.add_row(header.clone())?;


        let mut rows : Vec<(Option<T>,Vec<Option<String>>)> = Vec::new();

        let base_path = asgn.path.join(".info").join("score.toml");
        let scores = util::parse_from::<StatBlockSet>(&base_path)?;

        for member in context.members.iter() {
            let member_name = member.clone().into_string().unwrap();
            let stat_block = scores.get_block(&member_name);
            let mut row = vec![Some(member.clone().into_string().unwrap())];
            if let Some(stat_block) = stat_block {
                let score : Option<T> = stat_block.scores.get(rule_name)
                    .map(|v|v.to_string().parse::<T>().unwrap());
                for rule in ruleset.rules.iter() {
                    row.push(stat_block.scores.get(&rule.target).map(|v|v.to_string()));
                }
                rows.push((score,row));
            } else {
                row.resize_with(header.len(),||None);
                continue;
            };

        }

        rows.sort_by(|(a,_), (b,_)| {
            match (a,b) {
                (Some(a_score),Some(b_score)) => if up {
                        a_score.partial_cmp(b_score).unwrap()
                    } else {
                        a_score.partial_cmp(b_score).unwrap().reverse()
                    },
                (Some(_),None) => std::cmp::Ordering::Less,
                (None,Some(_)) => std::cmp::Ordering::Greater,
                (None,None)    => std::cmp::Ordering::Equal,
            }
        });

        for row in rows.iter().map(|(_,b)| b) {
            let row : Vec<String> = row.iter()
                .map(|entry| entry.clone().unwrap_or("None".to_string()))
                .collect();
            table.add_row(row)?;
        }

        print!("{table}");

        Ok(())
    }

    fn rank(asgn_name: &OsString, rule_name: &str, up: bool, context: &Context)
    -> Result<(),FailLog>
    {

        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        let Some(ruleset) = spec.score.as_ref() else {
            return Err(FailInfo::Custom(
                format!("Assignment '{}' has no scores to rank.",asgn_name.to_string_lossy()),
                format!("If you believe this assignment should have scores, contact the instructor.")
            ).into_log());
        };

        let kind = ruleset.rules.iter()
            .filter(|rule| rule.target == rule_name)
            .next()
            .map(|r| r.kind.clone())
            .ok_or(FailInfo::Custom(
                format!("Assignment '{}' does not have a '{}' score.",asgn_name.to_string_lossy(),rule_name),
                format!("If you believe this assignment should have this score, contact the instructor.")
            ).into_log())?;


        let Some(kind) = kind else {
            return Err(FailInfo::IOFail("No score kind given.".to_string()).into_log());
        };

        match kind.as_str() {
            "bool"  => Self::rank_specialized::<bool>(spec, ruleset, rule_name, up, context),
            "int"   => Self::rank_specialized::<i64>(spec, ruleset, rule_name, up, context),
            "float" => Self::rank_specialized::<f64>(spec, ruleset, rule_name, up, context),
            _       => Err(FailInfo::IOFail("Invalid score kind.".to_string()).into_log())
        }

    }


    fn submit(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        Self::verify_active(spec,context)?;

        let sub_dir = context.base_path.join(asgn_name).join(&context.user);

        let src_dir = context.cwd.clone();
        let mut log : FailLog = Default::default();
        for file_name in spec.file_list.iter() {
            let src_path = src_dir.join(file_name);
            let dst_path = sub_dir.join(file_name);
            if ! src_path.exists() {
                log.push(FailInfo::MissingFile(OsString::from(file_name)).into());
                continue;
            }
            if src_path.is_dir() {
                log.push(FailInfo::FileIsDir(OsString::from(file_name)).into());
                continue;
            }
            if ! src_path.is_file() {
                log.push(FailInfo::FileIsOther(OsString::from(file_name)).into());
                continue;
            }
            fs::copy(&src_path,&dst_path)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not copy file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)).into_log()
                })?;
            util::set_mode(&dst_path,0o777)?;
        }
        log.into_result()?;

        util::print_bold_hline();
        println!("{}",format!("Assignment '{}' submitted!",asgn_name.to_string_lossy()).green());

        let build_result = spec.run_on_submit(context,spec.build.as_ref(),&sub_dir,"Building",false);
        if build_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }

        let check_result = spec.run_on_submit(context,spec.check.as_ref(),&sub_dir,"Evaluating Checks",false);
        if check_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }

        let score_result = spec.run_on_submit(context,spec.score.as_ref(),&sub_dir,"Evaluating Scores",false);
        if score_result.map(|opt|opt.is_err()).unwrap_or(false) {
            return Ok(());
        }
        util::print_bold_hline();

        Ok(())
    }

    fn setup(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        Self::verify_active(spec,context)?;

        let setup_dir = context.base_path
            .join(asgn_name).join(".info").join("setup");

        if ! setup_dir.exists() {
            return Err(FailInfo::NoSetup(asgn_name.clone()).into());
        }

        let setup_name = format!("{}_setup",asgn_name.to_string_lossy());
        let dst_dir = util::make_fresh_dir(&context.cwd,&setup_name);

        StudentAct::copy_dir(dst_dir,setup_dir)
    }

    fn recover(asgn_name: &OsString, context: &Context) -> Result<(),FailLog>
    {

        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        Self::verify_active(spec,context)?;

        let sub_dir = context.base_path.join(asgn_name).join(&context.user);

        let recovery_name = format!("{}_recovery",asgn_name.to_string_lossy());
        let dst_dir = util::make_fresh_dir(&context.cwd,&recovery_name);

        fs::create_dir_all(&dst_dir)
            .map_err(|err| -> FailLog {FailInfo::IOFail(err.to_string()).into()})?;

        let mut log : FailLog = Default::default();
        for file_name in spec.file_list.iter() {
            let src_path = sub_dir.join(file_name);
            let dst_path = dst_dir.join(file_name);
            if ! src_path.exists() {
                log.push(FailInfo::MissingSub(OsString::from(file_name)).into());
                continue;
            }
            fs::copy(&src_path,&dst_path)
                .map_err(|err| {
                    FailInfo::IOFail(format!("could not copy file {} to {} : {}",
                    (&src_path).display(),(&dst_path).display(),err)).into_log()
                })?;
        }
        log.into_result()

    }


    fn alias(alias_name: &OsString, context: &Context) -> Result<(),FailLog>
    {
        let line = format!(
            "alias {}=\"{} {}\"",
            alias_name.to_string_lossy(),
            context.exe_path.clone().into_os_string().into_string().unwrap(),
            context.base_path.clone().into_os_string().into_string().unwrap()
        );
        bashrc_append_line(&line)?;
        println!("{}","Alias installed successfully.".yellow());
        println!("{}","The alias will take effect automatically for future shell sessions.".yellow());
        println!("{}","\nTo have it take effect for this shell session, run this command:".yellow());
        println!("{}","\n\nsource ~/.bashrc\n\n".green());
        Ok(())
    }

    fn details(asgn_name : &OsString, context: &Context) -> Result<(),FailLog> {
        let spec : &AsgnSpec = context.catalog.get(asgn_name)
            .ok_or(FailInfo::InvalidAsgn(asgn_name.clone()).into_log())?
            .as_ref().map_err(|err| err.clone() )?;

        if ! spec.visible {
            return Err(FailInfo::InvalidAsgn(asgn_name.clone()).into_log());
        }

        print!("{}", spec.details(context)?);
        Ok(())
    }

    pub fn execute(&self, context: &Context) -> Result<(),FailLog>
    {
        use StudentAct::*;
        match self {
            Other(act)             => act.execute(context),
            Submit  { asgn_name }  => Self::submit (asgn_name,context),
            Setup   { asgn_name }  => Self::setup  (asgn_name,context),
            Recover { asgn_name }  => Self::recover(asgn_name,context),
            Summary {}             => context.summary(),
            Details { asgn_name }  => Self::details(asgn_name,context),
            Grace   {asgn,ext}     => Self::grace(asgn,&context.user,*ext,context),
            Alias   { alias_name } => Self::alias(alias_name,context),
            RankAscending  {asgn,score} => Self::rank(asgn,score,true,context),
            RankDescending {asgn,score} => Self::rank(asgn,score,false,context),
        }
    }

}
