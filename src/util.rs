//! `mussh` utils

/// Pad the given string on the left with spaces if it is less than max length.
pub fn pad_left(s: &str, max: usize) -> String {
    let mut len = s.len();
    let mut res = String::new();

    while len < max {
        res.push(' ');
        len += 1;
    }

    res
}
