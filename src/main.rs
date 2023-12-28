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
#![warn(unused_crate_dependencies)] // This is useful for those times when you bring in a crate, then get rid of it when you realize it's the wrong solution.
#![warn(meta_variable_misuse)] // This caught a macro kleene-operator difference between definition and implementation, it might catch some other things
#![warn(unused_macro_rules)] // This caught some macro branches that weren't followed after a refactor
#![warn(unused_qualifications)] // This caught a few bits of code that looked bad
#![warn(unused_results)] // This one will be controversial, but I think it's useful. Most of the warnings should occur with map inserts, and a few of the removes. If it's something else, then I should think about it. It's easy to get around by adding a `_ = ` before the code (not a variable assignment, but a pattern assignment)
#![warn(unused_tuple_struct_fields)]
#![warn(variant_size_differences)]
#![allow(clippy::upper_case_acronyms)] // I disagree
#![allow(clippy::too_many_arguments)] // I'm fully aware that functions should be shorter, and can check this myself. But sometimes many parameters is just the simpler choice.
#![allow(clippy::mem_replace_with_default)] // I feel that std::mem::replace is more explicit in its meaning than std::mem::take. It would be different if the function was called std::mem::replace_default

// clippy lints as of clippy 1.72
#![warn(clippy::cargo_common_metadata)]
#![warn(clippy::assertions_on_result_states)]
#![warn(clippy::bool_to_int_with_if)]
#![warn(clippy::branches_sharing_code)]
#![warn(clippy::checked_conversions)]
#![warn(clippy::cloned_instead_of_copied)]
#![warn(clippy::cognitive_complexity)]
#![warn(clippy::collection_is_never_read)]
#![warn(clippy::default_trait_access)]
#![warn(clippy::derive_partial_eq_without_eq)]
#![warn(clippy::empty_structs_with_brackets)]
#![warn(clippy::equatable_if_let)]
#![warn(clippy::explicit_deref_methods)]
#![warn(clippy::explicit_into_iter_loop)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::float_cmp)]
#![warn(clippy::float_cmp_const)]
#![warn(clippy::format_push_string)]
#![warn(clippy::if_not_else)]
#![warn(clippy::implicit_clone)]
#![warn(clippy::imprecise_flops)]
#![warn(clippy::inconsistent_struct_constructor)]
#![warn(clippy::index_refutable_slice)]
#![warn(clippy::inefficient_to_string)]
#![warn(clippy::integer_division)]
#![warn(clippy::invalid_upcast_comparisons)]
#![warn(clippy::items_after_statements)]
#![warn(clippy::iter_on_empty_collections)]
#![warn(clippy::iter_on_single_items)]
#![warn(clippy::large_stack_arrays)]
#![warn(clippy::large_stack_frames)]
#![warn(clippy::large_types_passed_by_value)]
#![warn(clippy::manual_clamp)]
#![warn(clippy::manual_let_else)]
#![warn(clippy::manual_ok_or)]
#![warn(clippy::manual_string_new)]
#![warn(clippy::map_unwrap_or)]
#![warn(clippy::match_bool)]
#![warn(clippy::match_on_vec_items)]
#![warn(clippy::match_same_arms)]
#![warn(clippy::maybe_infinite_iter)]
#![warn(clippy::mismatching_type_param_order)]
#![warn(clippy::missing_const_for_fn)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::mixed_read_write_in_expression)]
#![warn(clippy::module_name_repetitions)]
#![warn(clippy::multiple_inherent_impl)]
#![warn(clippy::must_use_candidate)]
#![warn(clippy::mut_mut)]
#![warn(clippy::naive_bytecount)]
#![warn(clippy::needless_collect)]
#![warn(clippy::needless_continue)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::option_option)]
#![warn(clippy::or_fun_call)]
#![warn(clippy::redundant_clone)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::redundant_else)]
#![warn(clippy::redundant_type_annotations)]
#![warn(clippy::ref_binding_to_reference)]
#![warn(clippy::ref_binding_to_reference)]
#![warn(clippy::rest_pat_in_fully_bound_structs)]
#![warn(clippy::return_self_not_must_use)]
#![warn(clippy::same_functions_in_if_condition)]
#![warn(clippy::same_name_method)]
#![warn(clippy::shadow_unrelated)]
// TODO: #![warn(clippy::single_call_fn)]
#![warn(clippy::single_char_lifetime_names)]
#![warn(clippy::single_match_else)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_slice)]
#![warn(clippy::string_to_string)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::suspicious_operation_groupings)]
#![warn(clippy::suspicious_xor_used_as_pow)]
#![warn(clippy::todo)]
#![warn(clippy::trait_duplication_in_bounds)]
#![warn(clippy::trivially_copy_pass_by_ref)]
#![warn(clippy::try_err)]
#![warn(clippy::tuple_array_conversions)]
#![warn(clippy::type_repetition_in_bounds)]
#![warn(clippy::unicode_not_nfc)]
#![warn(clippy::uninlined_format_args)]
#![warn(clippy::unnecessary_struct_initialization)]
#![warn(clippy::unnecessary_wraps)]
#![warn(clippy::unnested_or_patterns)]
#![warn(clippy::unused_peekable)]
#![warn(clippy::unused_self)]
#![warn(clippy::use_self)]
#![warn(clippy::useless_let_if_seq)]
#![warn(clippy::zero_sized_map_values)]


use clap::Parser;

pub(crate) mod errors;
pub mod commands;
pub(crate) mod raster;
pub(crate) mod gdal_fixes;
pub(crate) mod geometry;
pub(crate) mod typed_map;
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
pub fn run<Arg, Args>(args: Args) -> Result<(),ProgramError> 
where 
    Arg: Clone + Into<std::ffi::OsString>, 
    Args: IntoIterator<Item = Arg> 
{
    let mut progress = ConsoleProgressBar::new();
    let command = Cosmopoeia::try_parse_from(args)?;
    command.run(&mut progress)?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    let args = std::env::args();
    // I could just return a Result<(),Box<dyn Error>> except the built-ins format that with debug instead of
    // display, so I don't get a good error message. At some point in the future, there's going to be a Report
    // trait that might be useful once it becomes stable. https://doc.rust-lang.org/stable/std/error/struct.Report.html#return-from-main
    match run(args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            std::process::ExitCode::FAILURE
        }
    }
}
