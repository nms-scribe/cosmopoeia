use clap::Parser;

mod errors;
mod commands;

pub use errors::ArgumentError;
pub use errors::CommandError;
pub use errors::ProgramError;

pub use commands::Tool;
pub use commands::Command;

#[macro_export]
macro_rules! command_help_template {
    () => {
        "{about-section}\n{usage-heading}\n{tab}{usage}\n\n{all-args}\n\nVersion: {version}\nAuthor:  {author}"
    };
}

#[macro_export]
macro_rules! subcommand_def {
    (#[doc = $about: literal] pub struct $name: ident $body: tt) => {
        #[derive(Args)]
        #[command(author,help_template = crate::command_help_template!())] 
        #[doc = $about]
        pub struct $name $body
                
    };
}

#[derive(Parser)]
#[command(author, version, long_about = None, help_template = command_help_template!())]
#[command(propagate_version = true)]
/// N M Sheldon's Fantasy Mapping Tools
pub struct CommandLine {

    #[command(subcommand)]
    pub command: Command

}


fn run<Arg, Args>(args: &mut Args) -> Result<(),ProgramError> 
where 
    Arg: Clone + Into<std::ffi::OsString>, 
    Args: Iterator<Item = Arg> 
{
    let command = CommandLine::try_parse_from(args)?.command;
    command.run()?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    let mut args = std::env::args();
    // I could just return a Result<(),Box<dyn Error>> except the built-ins format that with debug instead of
    // display, so I don't get a good error message. At some point in the future, there's going to be a Report
    // trait that might be useful once it becomes stable. https://doc.rust-lang.org/stable/std/error/struct.Report.html#return-from-main
    match run(&mut args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}",err);
            std::process::ExitCode::FAILURE
        }
    }
}