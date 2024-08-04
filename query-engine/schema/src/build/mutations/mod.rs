pub(crate) mod create_many;
pub(crate) mod create_one;

pub(crate) use create_many::{create_many, create_many_and_return};
pub(crate) use create_one::create_one;

use super::*;
