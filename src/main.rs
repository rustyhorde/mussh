// Copyright © 2016 libmussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! mussh - SSH Multiplexing
#![feature(crate_visibility_modifier, error_iter)]
#![deny(
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    array_into_iter,
    bare_trait_objects,
    broken_intra_doc_links,
    dead_code,
    deprecated,
    deprecated_in_future,
    elided_lifetimes_in_paths,
    ellipsis_inclusive_range_patterns,
    explicit_outlives_requirements,
    exported_private_dependencies,
    illegal_floating_point_literal_pattern,
    improper_ctypes,
    incomplete_features,
    indirect_structural_match,
    invalid_value,
    irrefutable_let_patterns,
    keyword_idents,
    late_bound_lifetime_arguments,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    // missing_doc_code_examples,
    missing_docs,
    mutable_borrow_reservation_conflict,
    no_mangle_generic_items,
    non_ascii_idents,
    non_camel_case_types,
    non_shorthand_field_patterns,
    non_snake_case,
    non_upper_case_globals,
    overlapping_range_endpoints,
    path_statements,
    // private_doc_tests,
    private_in_public,
    proc_macro_derive_resolution_fallback,
    redundant_semicolons,
    renamed_and_removed_lints,
    safe_packed_borrows,
    stable_features,
    trivial_bounds,
    trivial_casts,
    trivial_numeric_casts,
    type_alias_bounds,
    tyvar_behind_raw_pointer,
    unconditional_recursion,
    unknown_lints,
    unnameable_test_items,
    unreachable_code,
    unreachable_patterns,
    unreachable_pub,
    unsafe_code,
    // unstable_features,
    unstable_name_collisions,
    unused_allocation,
    unused_assignments,
    unused_attributes,
    unused_comparisons,
    unused_doc_comments,
    unused_extern_crates,
    unused_features,
    unused_import_braces,
    unused_imports,
    unused_labels,
    unused_lifetimes,
    unused_macros,
    unused_must_use,
    unused_mut,
    unused_parens,
    unused_qualifications,
    unused_results,
    unused_unsafe,
    unused_variables,
    variant_size_differences,
    where_clauses_object_safety,
    while_true
)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![doc(html_root_url = "https://docs.rs/mussh/3.0.0")]

mod error;
mod logging;
mod run;
mod subcmd;

use crate::error::{MusshErr, MusshErrKind};
use clap::ErrorKind;
use std::error::Error;
use std::process;

/// mussh entry point
fn main() {
    process::exit(match run::run() {
        Ok(_) => 0,
        Err(error) => error.source().and_then(is_lib_error).map_or_else(
            || {
                eprintln!("{}", error);
                1
            },
            |e| is_clap_help_or_version((&error, e)),
        ),
    })
}

fn is_lib_error<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a MusshErrKind> {
    error.downcast_ref::<MusshErrKind>()
}

fn is_clap_help_or_version(error_tuple: (&MusshErr, &MusshErrKind)) -> i32 {
    let (error, k_error) = error_tuple;
    let disp_err = || {
        eprintln!("{}", error);
        1
    };

    match k_error {
        MusshErrKind::Clap(e) => match e.kind {
            ErrorKind::HelpDisplayed => {
                eprintln!("{}", e.message);
                0
            }
            ErrorKind::VersionDisplayed => 0,
            _ => disp_err(),
        },
        _ => disp_err(),
    }
}
