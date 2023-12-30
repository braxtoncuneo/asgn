pub mod instructor;
pub mod grader;
pub mod student;
pub mod other;

use crate::{fail_info::FailLog, context::Context};

trait Action {
    fn execute(&self, context: &Context) -> Result<(), FailLog>;
}
