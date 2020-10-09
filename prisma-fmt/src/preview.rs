use crate::PreviewFeaturesOpts;
use datamodel::common::preview_features::{
    DATASOURCE_PREVIEW_FEATURES, DEPRECATED_GENERATOR_PREVIEW_FEATURES, GENERATOR_PREVIEW_FEATURES,
};

pub fn run(opts: PreviewFeaturesOpts) {
    let result = if opts.datasource_only {
        DATASOURCE_PREVIEW_FEATURES.to_vec()
    } else {
        let preview_features = GENERATOR_PREVIEW_FEATURES.to_vec();
        preview_features
            .iter()
            .filter(|pf| !DEPRECATED_GENERATOR_PREVIEW_FEATURES.contains(pf))
            .collect()
    };

    if result.len() == 0 {
        print!("[]")
    } else {
        let json = serde_json::to_string(&result).expect("Failed to render JSON");

        print!("{}", json)
    }
}
