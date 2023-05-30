use super::*;
use constants::aggregations::*;
use std::convert::identity;

pub(crate) fn model_object_type(ctx: &QuerySchema, model: Model) -> ObjectType<'_> {
    let model_id = model.id;
    let mut obj = ObjectType::new(Identifier::new_model(IdentifierType::Model(model.clone())), move || {
        let mut fields = compute_model_object_type_fields(ctx, &model);

        // Add _count field. Only include to-many fields.
        let relation_fields = model.fields().relation().filter(|f| f.is_list()).collect();

        append_opt(
            &mut fields,
            field::aggregation_relation_field(
                ctx,
                UNDERSCORE_COUNT,
                &model,
                relation_fields,
                |_, _| OutputType::non_list(OutputType::int()),
                identity,
            ),
        );

        fields
    });
    obj.model = Some(model_id);
    obj
}

/// Computes model output type fields.
fn compute_model_object_type_fields<'a>(ctx: &'a QuerySchema, model: &Model) -> Vec<OutputField<'a>> {
    model.fields().all().map(|f| field::map_output_field(ctx, f)).collect()
}
