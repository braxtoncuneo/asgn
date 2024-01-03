use std::path::PathBuf;

use structopt::StructOpt;

use crate::{context::Context, error::Error, fs_ext};

#[derive(Debug, StructOpt)]
#[structopt(
    name       = "asgn",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
pub struct OtherCmd {
    #[structopt(name = "base path")]
    _base_path: PathBuf, // Used only to consume the first CLI arg

    #[structopt(subcommand)]
    pub act: OtherAct,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "snake")]
pub enum OtherAct {
    #[structopt(about = "\"installs\" asgn by adding it to your path")]
    Install,
}

impl OtherAct {
    fn install(context: &Context) -> Result<(), Error> {
        let new_path = context.exe_path.parent().unwrap().to_str().unwrap();
        let path_append = format!("PATH=\"{new_path}:$PATH\"");
        fs_ext::bashrc_append_line(&path_append)
    }

    pub fn execute(&self, context: &Context) -> Result<(), Error> {
        match self {
            OtherAct::Install => Self::install(context),
        }
    }
}
