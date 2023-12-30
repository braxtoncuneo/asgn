#![feature(const_option)]

pub mod act;
pub mod asgn_spec;
pub mod context;
pub mod error;
pub mod util;
pub mod table;

use structopt::StructOpt;
use error::Error;
use context::{Context, Role};
use colored::Colorize;

fn main() {
    let mut args = std::env::args().skip(1).peekable();

    let Some(base_path) = args.next() else {
        println!("USAGE:");
        println!("asgn <base_path> <SUBCOMMAND>");
        return;
    };

    let ctx_try = Context::deduce(&base_path);

    let mut context = {
        let command = args.peek().map(String::as_str);

        match ctx_try {
            Ok(ctx) => {
                if command == Some("init") {
                    print!("{}", Error::Custom(
                        "Provided path is already the base path of a pre-existing, valid course directory.".to_owned(),
                        "Either clear out that directory, or use a different one.".to_owned()
                    ));
                    return;
                }
                ctx
            }
            Err(err) => return match command {
                Some("init") => if let Err(log) = context::init(&base_path) {
                    print!("{log}");
                },
                _ => println!("{err}"),
            }
        }
    };

    let result = match &context.role {
        Role::Instructor => act::instructor::InstructorCmd::from_args().act.execute(&mut context),
        Role::Grader => act::grader::GraderCmd::from_args().act.execute(&context),
        Role::Student => act::student::StudentCmd::from_args().act.execute(&context),
        Role::Other => {
            println!("{}", "! User not recognized as member of course.".red());
            println!("{}", "> If you believe you are a member, contact the instructor.".yellow());
            return;
        }
    };

    if let Err(log) = result {
        print!("{log}");
    }

    if
        context.role == Role::Instructor
        && args.peek().map(String::as_str) != Some("refresh")
    {
        print!("{}", context.all_catalog_errors());
    }
}
