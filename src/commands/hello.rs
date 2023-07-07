use clap::Args;

use super::Tool;
use crate::errors::CommandError;
use crate::subcommand_def;

subcommand_def!{
    /// Greets the user
    pub struct Hello {
        /// Name of user to greet.
        name: Option<String>
    }
}

impl Tool for Hello {

    fn run(self) -> Result<(),CommandError> {
        println!("Hello, {}!",self.name.unwrap_or_else(|| "User".to_owned()));
        Ok(())
    }
}
