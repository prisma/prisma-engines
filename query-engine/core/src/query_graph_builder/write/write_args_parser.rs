use super::*;
use crate::{
    constants::{json_null, operations},
    query_document::{ParsedInputMap, ParsedInputValue},
    ObjectTag,
};
use connector::{DatasourceFieldName, WriteArgs, WriteOperation};
use prisma_models::{
    CompositeFieldRef, Field, ModelRef, PrismaValue, RelationFieldRef, ScalarFieldRef, TypeIdentifier,
};
use std::{convert::TryInto, sync::Arc};

#[derive(Default, Debug)]
pub struct WriteArgsParser {
    pub args: WriteArgs,
    pub nested: Vec<(RelationFieldRef, ParsedInputMap)>,
}

impl WriteArgsParser {
    /// Creates a new set of WriteArgsParser. Expects the parsed input map from the respective data key, not the enclosing map.
    /// E.g.: { data: { THIS MAP } } from the `data` argument of a write query.
    #[tracing::instrument(name = "write_args_parser_from", skip(model, data_map))]
    pub fn from(model: &ModelRef, data_map: ParsedInputMap) -> QueryGraphBuilderResult<Self> {
        data_map.into_iter().try_fold(
            WriteArgsParser::default(),
            |mut args, (k, v): (String, ParsedInputValue)| {
                let field = model.fields().find_from_all(&k).unwrap();

                match field {
                    Field::Scalar(sf) if sf.is_list() => {
                        let write_op = parse_scalar_list(v)?;

                        args.args.insert(sf, write_op);
                    }
                    Field::Scalar(sf) => {
                        let write_op: WriteOperation = parse_scalar(sf, v)?;

                        args.args.insert(sf, write_op)
                    }

                    Field::Relation(ref rf) => match v {
                        ParsedInputValue::Single(PrismaValue::Null) => (),
                        _ => args.nested.push((Arc::clone(rf), v.try_into()?)),
                    },

                    Field::Composite(cf) => {
                        let write_op = parse_composite_writes(cf, v, &mut vec![])?;

                        args.args.insert(cf, write_op)
                    }
                };

                Ok(args)
            },
        )
    }
}

fn is_composite_envelope(map: &ParsedInputMap) -> bool {
    matches!(map.tag, Some(ObjectTag::CompositeEnvelope))
}

fn parse_scalar(sf: &ScalarFieldRef, v: ParsedInputValue) -> Result<WriteOperation, QueryGraphBuilderError> {
    match v {
        ParsedInputValue::Single(PrismaValue::Enum(e)) if sf.type_identifier == TypeIdentifier::Json => {
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

            let write_op = match operation.as_str() {
                operations::SET => WriteOperation::scalar_set(value),
                operations::INCREMENT => WriteOperation::scalar_add(value),
                operations::DECREMENT => WriteOperation::scalar_substract(value),
                operations::MULTIPLY => WriteOperation::scalar_multiply(value),
                operations::DIVIDE => WriteOperation::scalar_divide(value),
                operations::UNSET if sf.container().is_composite() => parse_composite_unset(value),
                _ => unreachable!("Invalid update operation"),
            };

            Ok(write_op)
        }
        _ => unreachable!(),
    }
}

fn parse_scalar_list(v: ParsedInputValue) -> QueryGraphBuilderResult<WriteOperation> {
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
    v: ParsedInputValue,
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
            if is_composite_envelope(&map) {
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
    envelope: ParsedInputMap,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteOperation> {
    let (op, value) = envelope.into_iter().next().unwrap();

    let write_op = match op.as_str() {
        // Everything in a set operation can only be plain values, no more nested operations.
        operations::SET => WriteOperation::composite_set(value.try_into()?),
        operations::UNSET => parse_composite_unset(value.try_into()?),
        // operations::PUSH => WriteOperation::composite_push(value.try_into()?),
        operations::UPDATE => parse_composite_updates(cf, value.try_into()?, path)?,
        _ => unimplemented!(),
    };

    Ok(write_op)
}

fn parse_composite_unset(pv: PrismaValue) -> WriteOperation {
    let should_unset = pv.as_boolean().unwrap();

    WriteOperation::composite_unset(*should_unset)
}

fn parse_composite_updates(
    cf: &CompositeFieldRef,
    map: ParsedInputMap,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteOperation> {
    let mut writes = vec![];

    for (k, v) in map {
        let field = cf.typ.find_field(&k).unwrap();

        let field_name: DatasourceFieldName = match field {
            Field::Scalar(sf) => sf.into(),
            Field::Composite(cf) => cf.into(),
            _ => unreachable!(),
        };

        let write_op = match field {
            Field::Scalar(sf) if sf.is_list() => parse_scalar_list(v),
            Field::Scalar(sf) => parse_scalar(&sf, v),
            Field::Composite(cf) => parse_composite_writes(&cf, v, &mut path.clone()),
            _ => unreachable!(),
        }?;

        writes.push((field_name, write_op));
    }

    Ok(WriteOperation::composite_update(writes))
}

fn extract_scalar_list_ops(map: ParsedInputMap) -> QueryGraphBuilderResult<WriteOperation> {
    let (operation, value) = map.into_iter().next().unwrap();
    let pv: PrismaValue = value.try_into()?;

    match operation.as_str() {
        operations::SET => Ok(WriteOperation::scalar_set(pv)),
        operations::PUSH => Ok(WriteOperation::scalar_add(pv)),
        _ => unreachable!("Invalid scalar list operation"),
    }
}
