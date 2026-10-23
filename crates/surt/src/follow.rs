mod chain;
mod editor;
pub(crate) mod prompt;
mod split;
pub(crate) mod suggestion;

pub(crate) use chain::follow;
use split::{SplitError, split_line};
