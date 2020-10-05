use datamodel::ast::reformat::Reformatter;
use std::{
  fs::{self, File},
  io::{self, BufWriter, Read},
};

use crate::{PreviewFeaturesOpts};
use datamodel::common::preview_features::{DATASOURCE_PREVIEW_FEATURES, GENERATOR_PREVIEW_FEATURES};
use datamodel::error::DatamodelError::GeneratorArgumentNotFound;

pub fn run(opts: PreviewFeaturesOpts) {
  let mut datamodel_string = String::new();

  let result = if opts.datasource_only {
    DATASOURCE_PREVIEW_FEATURES.to_vec()
  } else {
    GENERATOR_PREVIEW_FEATURES.to_vec()
  };

    if result.datasources.len() == 0 {
      print!("[]")
    } else {
        let json =
          serde_json::to_string(&result).expect("Failed to render JSON");

        print!("{}", json)
    }
}
