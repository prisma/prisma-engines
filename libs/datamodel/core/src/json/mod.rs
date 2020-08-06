//! This module contains the logic to turn models into their JSON/DMMF representation.
//!
//! The responsibilities of each sub module is:
//! * `dmmf`: Turns a `dml::Datamodel` into its JSON representation.
//! * `mcf`: Turns a collection of `configuration::Datasource` and `configuration::Generator` into a JSON representation.
//!
pub mod dmmf;
pub mod mcf;
