use crate::PreviewFeaturesOpts;
use datamodel::common::preview_features::{DATASOURCE, GENERATOR};

pub fn run(opts: PreviewFeaturesOpts) {
    let features = if opts.datasource_only {
        DATASOURCE.active_features()
    } else {
        GENERATOR.active_features()
    };

    if features.is_empty() {
        print!("[]")
    } else {
        let json = serde_json::to_string(&features).expect("Failed to render JSON");

        print!("{}", json)
    }
}
