use crate::{ast, diagnostics::DatamodelError};
use crate::{
    common::preview_features::*,
    diagnostics::{DatamodelWarning, Diagnostics},
};
use itertools::Itertools;

impl ast::WithAttributes for Vec<ast::Attribute> {
    fn attributes(&self) -> &Vec<ast::Attribute> {
        self
    }
}

pub fn parse_and_validate_preview_features(
    preview_features: Vec<String>,
    feature_map: &FeatureMap,
    span: ast::Span,
) -> (Vec<PreviewFeature>, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let mut features = vec![];

    for feature_str in preview_features {
        let feature_opt = PreviewFeature::parse_opt(&feature_str);
        match feature_opt {
            Some(feature) if feature_map.is_deprecated(&feature) => {
                features.push(feature);
                diagnostics.push_warning(DatamodelWarning::new_deprecated_preview_feature_warning(
                    &feature_str,
                    span,
                ))
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

    (features, diagnostics)
}
