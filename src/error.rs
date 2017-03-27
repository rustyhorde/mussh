// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `mussh` errors
error_chain!{
    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        Config {
            description("Could not find valid mussh.toml file!")
            display("Could not find valid mussh.toml file!")
        }
        InvalidCmd(reason: String) {
            description("Invalid command specified!")
            display("Invalid command specified! {}", reason)
        }
        InvalidHosts {
            description("Invalid hosts specified!")
            display("Invalid hosts specified!")
        }
    }
}
