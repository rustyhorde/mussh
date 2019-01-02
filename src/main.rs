// Copyright (c) 2016, 2018 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! mussh - SSH Multiplexing
#![feature(crate_visibility_modifier, try_from)]
#![deny(
    clippy::all,
    clippy::pedantic,
    macro_use_extern_crate,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused
)]
#![warn(
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    bare_trait_objects,
    box_pointers,
    elided_lifetimes_in_paths,
    ellipsis_inclusive_range_patterns,
    keyword_idents,
    question_mark_macro_sep,
    single_use_lifetimes,
    unreachable_pub,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]
#![allow(clippy::module_name_repetitions)]
#![doc(html_root_url = "https://docs.rs/mussh/3.0.0")]

mod error;
mod logging;
mod run;

use clap::ErrorKind;
use std::error::Error;
use std::process;

/// mussh entry point
fn main() {
    match run::run() {
        Ok(_) => process::exit(0),
        Err(error) => {
            if let Some(cause) = error.source() {
                if let Some(err) = cause.downcast_ref::<clap::Error>() {
                    let kind = err.kind;
                    eprintln!("{}", err.message);
                    match kind {
                        ErrorKind::HelpDisplayed | ErrorKind::VersionDisplayed => process::exit(0),
                        _ => process::exit(1),
                    }
                } else {
                    eprintln!("{}", error.description());

                    if let Some(cause) = error.source() {
                        eprintln!(": {}", cause);
                    }
                    process::exit(1);
                }
            } else {
                eprintln!("{}", error.description());

                if let Some(cause) = error.source() {
                    eprintln!(": {}", cause);
                }
                process::exit(1);
            }
        }
    }
}
