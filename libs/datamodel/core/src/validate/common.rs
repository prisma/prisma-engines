use crate::{
    ast,
    common::preview_features::*,
    diagnostics::{DatamodelError, DatamodelWarning, Diagnostics},
};
use itertools::Itertools;

pub fn parse_and_validate_preview_features(
    preview_features: Vec<String>,
    feature_map: &FeatureMap,
    span: ast::Span,
    diagnostics: &mut Diagnostics,
) -> Vec<PreviewFeature> {
    let mut features = vec![];

    for feature_str in preview_features {
        let feature_opt = PreviewFeature::parse_opt(&feature_str);
        match feature_opt {
            Some(feature) if feature_map.is_deprecated(&feature) => {
                features.push(feature);
                diagnostics.push_warning(DatamodelWarning::new_feature_deprecated(&feature_str, span));
            }

            Some(feature) if !feature_map.is_valid(&feature) => {
                diagnostics.push_error(DatamodelError::new_preview_feature_not_known_error(
                    &feature_str,
                    feature_map.active_features().iter().map(ToString::to_string).join(", "),
                    span,
                ))
            }

            Some(feature) => features.push(feature),

            None => diagnostics.push_error(DatamodelError::new_preview_feature_not_known_error(
                &feature_str,
                feature_map.active_features().iter().map(ToString::to_string).join(", "),
                span,
            )),
        }
    }

    features
}
