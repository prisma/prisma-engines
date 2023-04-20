use crate::{introspection_pair::EnumPair, sanitize_datamodel_names};
use schema_connector::{warnings as generators, Warnings};

/// Analyze and generate warnigs from an enum.
pub(super) fn generate_warnings(r#enum: EnumPair<'_>, warnings: &mut Warnings) {
    if r#enum.name_from_psl() {
        let warning = generators::warning_enriched_with_map_on_enum(&[generators::Enum::new(&r#enum.name())]);
        warnings.push(warning);
    }

    if r#enum.uses_duplicate_name() {
        warnings.duplicate_names.push(generators::TopLevelItem {
            r#type: generators::TopLevelType::Enum,
            name: r#enum.name().to_string(),
        });
    }

    if r#enum.description().is_some() {
        warnings.objects_with_comments.push(generators::Object {
            r#type: "enum",
            name: r#enum.name().to_string(),
        });
    }

    for variant in r#enum.variants() {
        if variant.name().is_empty() {
            let value = variant
                .mapped_name()
                .map(String::from)
                .unwrap_or_else(|| variant.name().to_string());

            let warning = generators::EnumAndValue {
                enm: r#enum.name().to_string(),
                value,
            };

            warnings.enum_values_with_empty_names.push(warning);
        }

        if variant.name().is_empty() || sanitize_datamodel_names::needs_sanitation(&variant.name()) {
            let warning = generators::EnumAndValue {
                enm: r#enum.name().to_string(),
                value: variant.name().to_string(),
            };

            warnings.enum_values_with_empty_names.push(warning);
        } else if variant.name_from_psl() {
            warnings.remapped_values.push(generators::EnumAndValue {
                value: variant.name().to_string(),
                enm: r#enum.name().to_string(),
            });
        }
    }
}
