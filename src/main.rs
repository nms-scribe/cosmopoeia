/*!
Cosmopoeia is a tool for generating fantasy worlds in the form of a geopackage file. For instructions, see the wiki.
*/
use clap::Parser;

pub(crate) mod errors;
pub mod commands;
pub(crate) mod raster;
pub(crate) mod gdal_fixes;
pub(crate) mod world_map;
pub(crate) mod utils;
pub(crate) mod progress;
pub(crate) mod algorithms;
#[cfg(test)] mod test;

use errors::ProgramError;

use commands::Cosmopoeia;
use progress::ConsoleProgressBar;

/**
Runs Cosmopoeia with arbitrary arguments. The first item in the arguments will be ignored. All output will be printed to Stdout or Stderr.
*/
pub fn run<Arg, Args>(args: &mut Args) -> Result<(),ProgramError> 
where 
    Arg: Clone + Into<std::ffi::OsString>, 
    Args: Iterator<Item = Arg> 
{
    let mut progress = ConsoleProgressBar::new();
    let command = Cosmopoeia::try_parse_from(args)?;
    command.run(&mut progress)?;
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
