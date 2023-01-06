
pub mod instructor;
pub mod grader;
pub mod student;
pub mod other;



use crate::{
    fail_info::
    {
        FailLog,
    },
    context::Context,
};



//#[structopt(parse(from_os_str))]



trait Action
{
    fn execute(&self,context: &Context) -> Result<(),FailLog>;
}



enum Role
{
    Instructor,
    Grader,
    Student,
    Other,
}


impl Role
{

    fn determine(context: &Context) -> Result<Self,FailLog>
    {
        todo!()
    }

}



