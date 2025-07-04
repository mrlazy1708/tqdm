//! Progress bar style enumeration
//!
//! - `ASCII`: Pure ASCII bar with `"0123456789#"`
//! - `Block`: Common bar with unicode characters `" ▏▎▍▌▋▊▉█"`
//! - `Balloon`: Simulate balloon explosion with `".oO@*"`. Inspired by [stackoverflow](https://stackoverflow.com/a/2685509/17570263)
//! - `Pacman`: Inspired by Arch Linux ILoveCandy
//! - `Custom`: Create a custom progressbar style
//!
//! Other styles are open for [contribution](https://github.com/mrlazy1708/tqdm/issues/1).

pub enum Style {
    ASCII,
    Block,
    Balloon,
    Pacman,
    Custom(String)
}

impl Default for Style {
    fn default() -> Self {
        Style::Block
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Style::ASCII => "0123456789#",
            Style::Block => " ▏▎▍▌▋▊▉█",
            Style::Balloon => ".oO@*",
            Style::Pacman => "C-",
            Style::Custom(n) => &n[..],
        })
    }
}
