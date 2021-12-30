use super::*;
use prisma_models::CompositeFieldRef;

/// Composite input for creates. // Parent is create?
pub(crate) fn composite_create_input_fields(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    // for field in model.fields().composite() input_field()
    vec![]
}

pub(crate) fn composite_input_field(
    ctx: &mut BuilderContext,
    cf: &CompositeFieldRef,
    parent_is_create: bool,
) -> InputField {
    let typ = composite_writes_input_envelope_object_type(ctx, cf, parent_is_create);

    // input_field(cf.name.clone(), typ).nullable_if(!cf.is_list() && !cf.is_required())
    todo!()
}

fn composite_writes_input_envelope_object_type(
    ctx: &mut BuilderContext,
    cf: &CompositeFieldRef,
    parent_is_create: bool,
) -> ObjectTypeWeakRef {
    // `parent_is_create` excludes update operations (or any in-place mutating operations).

    if cf.is_list() {
        // list_envelope()
    } else if !cf.is_required() {
        // set
        // upsert
        // unset
    } else {
        // set
        // update
    }

    todo!()
}

fn list_envelope(ctx: &mut BuilderContext, cf: &CompositeFieldRef, parent_is_create: bool) -> InputObjectTypeWeakRef {
    let name = if parent_is_create {
        format!("{}ListCreateInput", cf.typ.name)
    } else {
        format!("{}ListInput", cf.typ.name)
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    // set
    // push
    // updateMany
    // deleteMany

    todo!()
}

// Set is likely the same input object as set.
fn set_object_type(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let name = format!("{}SetInput", cf.typ.name);
    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    // set fields etc.

    if cf.is_list() {
        //
    } else if !cf.is_required() {
        // set
        // upsert
        // unset
    } else {
        // set
        // update
    }

    todo!()
}

/// Reusable input object type for composites, similar to the base create/update model inputs.
/// `with_defaults`:
fn input_object_type(ctx: &mut BuilderContext, cf: &CompositeFieldRef, with_defaults: bool) -> InputObjectTypeWeakRef {
    let name = if with_defaults {
        format!("{}Input", cf.typ.name)
    } else {
        format!("{}WithoutDefaultsInput", cf.typ.name)
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let fields = cf
        .typ
        .fields()
        .into_iter()
        .map(|field| {
            match field {
                ModelField::Scalar(sf) => {
                    let default = if with_defaults { sf.default_value.clone() } else { None };

                    if sf.is_list() {
                        // super::input_fields::scalar_list_input_field_mapper(
                        //     ctx,
                        // )
                    }

                    todo!()
                }
                ModelField::Composite(_) => todo!(),
                ModelField::Relation(_) => unreachable!(), // Composites do not have relation fields.
            }
            todo!()
        })
        .collect();

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}
