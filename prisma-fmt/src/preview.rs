use crate::PreviewFeaturesOpts;
use datamodel::common::preview_features::{
    DATASOURCE_PREVIEW_FEATURES, DEPRECATED_GENERATOR_PREVIEW_FEATURES, GENERATOR_PREVIEW_FEATURES,
};

pub fn run(opts: PreviewFeaturesOpts) {
    let result: Vec<&str> = if opts.datasource_only {
        DATASOURCE_PREVIEW_FEATURES.to_vec()
    } else {
        GENERATOR_PREVIEW_FEATURES
            .iter()
            .filter(|pf| !DEPRECATED_GENERATOR_PREVIEW_FEATURES.contains(pf))
            .map(|&x| x)
            .collect()
    };

    if result.is_empty() {
        print!("[]")
    } else {
        let json = serde_json::to_string(&result).expect("Failed to render JSON");

        print!("{}", json)
    }
}
