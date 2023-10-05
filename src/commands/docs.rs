use clap::Args;
use clap_markdown::print_help_markdown;

use crate::Cosmopoeia;
use crate::commands::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::progress::ProgressObserver;

subcommand_def!{
    /// Writes documentation to a folder
    #[command(hide=true)]
    pub struct Docs {

    }
}

impl Task for Docs {
    fn run<Progress: ProgressObserver>(self, _: &mut Progress) -> Result<(),CommandError> {
        print_help_markdown::<Cosmopoeia>();
        Ok(())
    }
}
