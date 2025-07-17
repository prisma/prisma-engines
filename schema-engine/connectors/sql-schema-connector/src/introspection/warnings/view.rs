use crate::introspection::introspection_pair::ViewPair;
use schema_connector::{Warnings, warnings as generators};

/// Analyze and generate warnigs from a view.
pub(super) fn generate_warnings(view: ViewPair<'_>, warnings: &mut Warnings) {
    if view.id().and_then(|id| id.name()).is_some() {
        warnings.reintrospected_id_names_in_view.push(generators::View {
            view: view.name().to_string(),
        });
    }

    if !view.has_usable_identifier() && !view.ignored_in_psl() {
        warnings.views_without_identifiers.push(generators::View {
            view: view.name().to_string(),
        });
    }

    if view.uses_duplicate_name() {
        warnings.duplicate_names.push(generators::TopLevelItem {
            r#type: generators::TopLevelType::View,
            name: view.name().to_string(),
        })
    }

    if view.remapped_name() {
        warnings.remapped_views.push(generators::View {
            view: view.name().to_string(),
        });
    }

    if view.description().is_some() {
        warnings.objects_with_comments.push(generators::Object {
            r#type: "view",
            name: view.name().to_string(),
        })
    }

    for field in view.scalar_fields() {
        if field.remapped_name_from_psl() {
            let mf = generators::ViewAndField {
                view: view.name().to_string(),
                field: field.name().to_string(),
            };

            warnings.remapped_fields_in_view.push(mf);
        }

        if field.is_unsupported() {
            let mf = generators::ViewAndFieldAndType {
                view: view.name().to_string(),
                field: field.name().to_string(),
                r#type: field.prisma_type().to_string(),
            };

            warnings.unsupported_types_in_view.push(mf)
        }

        if field.remapped_name_empty() {
            let mf = generators::ViewAndField {
                view: view.name().to_string(),
                field: field.name().to_string(),
            };

            warnings.fields_with_empty_names_in_view.push(mf);
        }

        if field.description().is_some() {
            warnings.objects_with_comments.push(generators::Object {
                r#type: "field",
                name: format!("{}.{}", view.name(), field.name()),
            })
        }
    }

    for field in view.relation_fields() {
        if field.reintrospected_relation() {
            warnings.reintrospected_relations.push(generators::Model {
                model: field.prisma_type().into_owned(),
            });
        }
    }
}
