
use structopt::StructOpt;


#[derive(Debug,StructOpt)]
#[structopt(
    name       = "asgn",
    author     = "Braxton Cuneo",
    about      = "A program for managing code assignments",
    version    = "0.0.1",
    rename_all = "snake",
)]
enum OtherCmd {}
