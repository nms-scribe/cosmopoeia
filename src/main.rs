/*!
Cosmopoeia is a tool for generating fantasy worlds in the form of a geopackage file. For instructions, see the wiki.
*/

//#![warn(non_exhaustive_omitted_patterns)] // FUTURE: unstable, but it looks useful
//#![warn(private_bounds)] // FUTURE: unstable, but it looks useful
//#![warn(private_interfaces)] // FUTURE: unstable, but it looks useful
//#![warn(unnameable_types)] // FUTURE: unstable, but it looks useful
#![warn(noop_method_call)] // This hasn't caught anything yet, but it is something I've worried about.
#![warn(single_use_lifetimes)] // This caught a few places where I didn't need to specify lifetimes but did.
#![warn(unused_lifetimes)] // As did this
#![warn(trivial_numeric_casts)] // This caught some 'as' statements which were leftover from a previous refactor
#![warn(unreachable_pub)] // This caught some 'pub' declarations that weren't necessary
#![warn(unused_crate_dependencies)] // This hasn't caught anything yet, but I do want to be told if I need to get rid of a crate.
#![warn(meta_variable_misuse)] // This caught a macro kleene-operator difference between definition and implementation, it might catch some other things
#![warn(unused_macro_rules)] // This caught some macro branches that weren't followed after a refactor
#![warn(unused_qualifications)] // This caught a few bits of code that looked bad
#![warn(unused_results)] // This one will be controversial, but I think it's useful. Most of the warnings should occur with map inserts, and a few of the removes. If it's something else, then I should think about it. It's easy to get around by adding a `_ = ` before the code (not a variable assignment, but a pattern assignment)
#![warn(unused_tuple_struct_fields)]
#![warn(variant_size_differences)]

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
