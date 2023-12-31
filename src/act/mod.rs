pub mod instructor;
pub mod grader;
pub mod student;
pub mod other;

use crate::{error::ErrorLog, context::Context};

trait Action {
    fn execute(&self, context: &Context) -> Result<(), ErrorLog>;
}
