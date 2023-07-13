
use clap::Subcommand;

use crate::errors::CommandError;

mod dev;

// NOTE: Further 'use' statements in the command macro below

pub trait Task {

    fn run(self) -> Result<(),CommandError>;

}

macro_rules! command {
    ($($command_mod: ident::$command_name: ident;)*) => {

        $(
            pub use $command_mod::$command_name;
        )*

        #[derive(Subcommand)]
        pub enum Command {
            $(
                $command_name($command_name)
            ),*
        }

        impl Task for Command {

            fn run(self) -> Result<(),CommandError> {
                match self {
                    $(Self::$command_name(a) => a.run()),*
                }
            }

        }
    };
}

command!{
    dev::DevGdalVersion;
    dev::DevGdalInfo;
    dev::DevGdalDrivers;
    dev::DevPointsFromHeightmap;
}


