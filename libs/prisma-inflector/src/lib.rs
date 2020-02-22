mod categories;
mod exceptions;
mod inflector;
mod rules;

use once_cell::sync::Lazy;
use inflector::{Inflector, Mode};

static DEFAULT: Lazy<Inflector> = Lazy::new(|| Inflector::new(Mode::Anglicized));
static CLASSICAL: Lazy<Inflector> = Lazy::new(|| Inflector::new(Mode::Classical));

/// Default inflector, anglicized mode.
pub fn default() -> &'static Inflector {
    &DEFAULT
}

/// Inflector, classical mode.
pub fn classical() -> &'static Inflector {
    &CLASSICAL
}
