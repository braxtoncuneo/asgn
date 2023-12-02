
use structopt::StructOpt;
use std::ffi::OsString;

use crate::
{
    context::Context,
    fail_info::
    {
        FailLog,
    },
    util::bashrc_append_line,
};

#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct OtherCmd
{
    #[structopt(name = "base path")]
    base_path : OsString,

    #[structopt(subcommand)]
    pub act: OtherAct,
}

#[derive(Debug,StructOpt)]
#[structopt(rename_all = "snake")]
pub enum OtherAct {
    #[structopt(about = "\"installs\" asgn by adding it to your path")]
    Install{},
}


impl OtherAct
{

    fn install(context: &Context) -> Result<(),FailLog> {
        let path_append : OsString = format!(
            "PATH=\"{}:$PATH\"",
            context.exe_path.parent().unwrap().display()
        ).into();
        bashrc_append_line(path_append.to_string_lossy())
    }

    pub fn execute(&self, context: &Context) -> Result<(),FailLog>
    {
        use OtherAct::*;
        match self {
            Install  {} => Self::install (context),
        }
    }
}

