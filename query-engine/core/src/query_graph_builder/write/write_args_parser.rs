use super::*;
use crate::query_document::{ParsedInputMap, ParsedInputValue};
use query_structure::{
    CompositeFieldRef, DatasourceFieldName, Field, Model, PrismaValue, RelationFieldRef, ScalarFieldRef,
    TypeIdentifier, WriteArgs, WriteOperation,
};
use schema::constants::{args, json_null, operations};
use std::{borrow::Cow, convert::TryInto};

#[derive(Debug)]
pub struct WriteArgsParser<'a> {
    pub(crate) args: WriteArgs,
    pub(crate) nested: Vec<(RelationFieldRef, ParsedInputMap<'a>)>,
}

impl<'a> WriteArgsParser<'a> {
    /// Creates a new set of WriteArgsParser. Expects the parsed input map from the respective data key, not the enclosing map.
    /// E.g.: { data: { THIS MAP } } from the `data` argument of a write query.
    pub(crate) fn from(model: &Model, data_map: ParsedInputMap<'a>) -> QueryGraphBuilderResult<Self> {
        data_map.into_iter().try_fold(
            WriteArgsParser {
                args: WriteArgs::new_empty(crate::executor::get_request_now()),
                nested: Default::default(),
            },
            |mut args, (k, v): (Cow<'_, str>, ParsedInputValue<'_>)| {
                let field = model.fields().find_from_all(&k).unwrap();

                match field {
                    Field::Scalar(sf) if sf.is_list() => {
                        let write_op = parse_scalar_list(v)?;

                        args.args.insert(&sf, write_op);
                    }
                    Field::Scalar(sf) => {
                        let write_op: WriteOperation = parse_scalar(&sf, v)?;

                        args.args.insert(&sf, write_op)
                    }

                    Field::Relation(ref rf) => match v {
                        ParsedInputValue::Single(PrismaValue::Null) => (),
                        _ => args.nested.push((rf.clone(), v.try_into()?)),
                    },

                    Field::Composite(cf) => {
                        let write_op = parse_composite_writes(&cf, v, &mut vec![])?;

                        args.args.insert(&cf, write_op)
                    }
                };

                Ok(args)
            },
        )
    }

    pub(crate) fn has_nested_operation(model: &Model, data_map: &ParsedInputMap<'a>) -> bool {
        data_map
            .iter()
            .any(|(field_name, _)| model.fields().find_from_relation_fields(field_name).is_ok())
    }
}

fn parse_scalar(sf: &ScalarFieldRef, v: ParsedInputValue<'_>) -> Result<WriteOperation, QueryGraphBuilderError> {
    match v {
        ParsedInputValue::Single(PrismaValue::Enum(e)) if sf.type_identifier() == TypeIdentifier::Json => {
            let val = match e.as_str() {
                json_null::DB_NULL => PrismaValue::Null,
                json_null::JSON_NULL => PrismaValue::Json("null".to_owned()),
                _ => unreachable!(), // Validation guarantees correct enum values.
            };

            Ok(WriteOperation::scalar_set(val))
        }
        ParsedInputValue::Single(v) => Ok(WriteOperation::scalar_set(v)),
        ParsedInputValue::Map(map) => {
            let (operation, value) = map.into_iter().next().unwrap();
            let value: PrismaValue = value.try_into()?;

            let write_op = match operation.as_ref() {
                operations::SET => WriteOperation::scalar_set(value),
                operations::UNSET => WriteOperation::scalar_unset(*value.as_boolean().unwrap()),
                operations::INCREMENT => WriteOperation::scalar_add(value),
                operations::DECREMENT => WriteOperation::scalar_substract(value),
                operations::MULTIPLY => WriteOperation::scalar_multiply(value),
                operations::DIVIDE => WriteOperation::scalar_divide(value),
                _ => unreachable!("Invalid update operation"),
            };

            Ok(write_op)
        }
        _ => unreachable!(),
    }
}

fn parse_scalar_list(v: ParsedInputValue<'_>) -> QueryGraphBuilderResult<WriteOperation> {
    match v {
        ParsedInputValue::List(_) => {
            let set_value: PrismaValue = v.try_into()?;

            Ok(WriteOperation::scalar_set(set_value))
        }
        ParsedInputValue::Map(map) => extract_scalar_list_ops(map),
        _ => unreachable!(),
    }
}

fn parse_composite_writes(
    cf: &CompositeFieldRef,
    v: ParsedInputValue<'_>,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteOperation> {
    match v {
        // Null-set operation.
        ParsedInputValue::Single(PrismaValue::Null) => Ok(WriteOperation::composite_set(PrismaValue::Null)),

        // Set list shorthand operation (can only be objects).
        ParsedInputValue::List(_) => {
            let list: PrismaValue = v.try_into()?;

            Ok(WriteOperation::composite_set(list))
        }

        // One of:
        // - Operation envelope with further actions nested.
        // - Single object set shorthand.
        ParsedInputValue::Map(map) => {
            if map.is_composite_envelope() {
                parse_composite_envelope(cf, map, path)
            } else {
                let pv: PrismaValue = ParsedInputValue::Map(map).try_into()?;

                Ok(WriteOperation::composite_set(pv))
            }
        }
        _ => unreachable!(),
    }
}

fn parse_composite_envelope(
    cf: &CompositeFieldRef,
    envelope: ParsedInputMap<'_>,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteOperation> {
    let (op, value) = envelope.into_iter().next().unwrap();

    let write_op = match op.as_ref() {
        // Everything in a set operation can only be plain values, no more nested operations.
        operations::SET => WriteOperation::composite_set(value.try_into()?),
        operations::PUSH => WriteOperation::composite_push(value.try_into()?),
        operations::UNSET => parse_composite_unset(value.try_into()?),
        operations::UPDATE => parse_composite_updates(cf, value.try_into()?, path)?,
        operations::UPSERT => parse_composite_upsert(cf, value.try_into()?, path)?,
        operations::UPDATE_MANY => parse_composite_update_many(cf, value.try_into()?, path)?,
        operations::DELETE_MANY => parse_composite_delete_many(cf, value.try_into()?)?,
        _ => unimplemented!(),
    };

    Ok(write_op)
}

fn parse_composite_update_many(
    cf: &CompositeFieldRef,
    mut value: ParsedInputMap<'_>,
    path: &mut [DatasourceFieldName],
) -> QueryGraphBuilderResult<WriteOperation> {
    let where_map: ParsedInputMap<'_> = value.swap_remove(args::WHERE).unwrap().try_into()?;
    let filter = extract_filter(where_map, cf.typ())?;

    let update_map: ParsedInputMap<'_> = value.swap_remove(args::DATA).unwrap().try_into()?;
    let update = parse_composite_updates(cf, update_map, path)?
        .try_into_composite()
        .unwrap();

    Ok(WriteOperation::composite_update_many(filter, update))
}

fn parse_composite_delete_many(
    cf: &CompositeFieldRef,
    mut value: ParsedInputMap<'_>,
) -> QueryGraphBuilderResult<WriteOperation> {
    let where_map: ParsedInputMap<'_> = value.swap_remove(args::WHERE).unwrap().try_into()?;
    let filter = extract_filter(where_map, cf.typ())?;

    Ok(WriteOperation::composite_delete_many(filter))
}

fn parse_composite_upsert(
    cf: &CompositeFieldRef,
    mut value: ParsedInputMap<'_>,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteOperation> {
    let set = value.swap_remove(operations::SET).unwrap();
    let set = parse_composite_writes(cf, set, path)?.try_into_composite().unwrap();
    let update: ParsedInputMap<'_> = value.swap_remove(operations::UPDATE).unwrap().try_into()?;
    let update = parse_composite_updates(cf, update, path)?.try_into_composite().unwrap();

    Ok(WriteOperation::composite_upsert(set, update))
}

fn parse_composite_unset(pv: PrismaValue) -> WriteOperation {
    let should_unset = pv.as_boolean().unwrap();

    WriteOperation::composite_unset(*should_unset)
}

fn parse_composite_updates(
    cf: &CompositeFieldRef,
    map: ParsedInputMap<'_>,
    path: &mut [DatasourceFieldName],
) -> QueryGraphBuilderResult<WriteOperation> {
    let mut writes = vec![];

    for (k, v) in map {
        let typ = cf.typ();
        let field = typ.find_field(&k).unwrap();

        let field_name: DatasourceFieldName = match &field {
            Field::Scalar(sf) => sf.into(),
            Field::Composite(cf) => cf.into(),
            _ => unreachable!(),
        };

        let write_op = match field {
            Field::Scalar(sf) if sf.is_list() => parse_scalar_list(v),
            Field::Scalar(sf) => parse_scalar(&sf, v),
            Field::Composite(cf) => parse_composite_writes(&cf, v, &mut path.to_owned()),
            _ => unreachable!(),
        }?;

        writes.push((field_name, write_op));
    }

    Ok(WriteOperation::composite_update(writes))
}

fn extract_scalar_list_ops(map: ParsedInputMap<'_>) -> QueryGraphBuilderResult<WriteOperation> {
    let (operation, value) = map.into_iter().next().unwrap();
    let pv: PrismaValue = value.try_into()?;

    match operation.as_ref() {
        operations::SET => Ok(WriteOperation::scalar_set(pv)),
        operations::PUSH => Ok(WriteOperation::scalar_add(pv)),
        _ => unreachable!("Invalid scalar list operation"),
    }
}
