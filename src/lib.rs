#[macro_use]
extern crate serde_json;

mod diff;
pub use crate::diff::JsonDiff;

mod colorize;
pub use crate::colorize::colorize_to_array;

#[cfg(feature = "colorize")]
pub use crate::colorize::colorize;
