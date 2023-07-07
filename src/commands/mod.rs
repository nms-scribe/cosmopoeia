
use clap::Subcommand;

use crate::errors::CommandError;

// NOTE: Further 'mod' and 'use' statements in the command macro below

pub trait Tool {

    fn run(self) -> Result<(),CommandError>;

}

macro_rules! command {
    ($($command_mod: ident::$command_name: ident);*) => {

        $(
            mod $command_mod;
        )*

        $(
            pub use $command_mod::$command_name;
        )*

        #[derive(Subcommand)]
        pub enum Command {
            $(
                $command_name($command_name)
            ),*
        }

        impl Tool for Command {

            fn run(self) -> Result<(),CommandError> {
                match self {
                    $(Self::$command_name(a) => a.run()),*
                }
            }

        }
    };
}

command!{
    hello::Hello
}


