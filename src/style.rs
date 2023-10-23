//! Progress bar style enumeration
//!
//! - `ASCII`: Pure ASCII bar with `"0123456789#"`
//! - `Block`: Common bar with unicode characters `" ▏▎▍▌▋▊▉█"`
//! - `Balloon`: Simulate balloon explosion with `".oO@*"`. Inspired by [stackoverflow](https://stackoverflow.com/a/2685509/17570263)
//!
//! Other styles are open for [contribution](https://github.com/mrlazy1708/tqdm/issues/1).
use std::string::{String, ToString};

pub enum Style {
    ASCII,
    Block,
    Balloon,
}

impl Default for Style {
    fn default() -> Self {
        Style::ASCII
    }
}

impl ToString for Style {
    fn to_string(&self) -> String {
        String::from(match self {
            Style::ASCII => "0123456789#",
            Style::Block => " ▏▎▍▌▋▊▉█",
            Style::Balloon => ".oO@*",
        })
    }
}
