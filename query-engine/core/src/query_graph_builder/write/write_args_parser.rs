use super::*;
use crate::{
    constants::{json_null, operations},
    query_document::{ParsedInputMap, ParsedInputValue},
    ObjectTag,
};
use connector::{DatasourceFieldName, NestedWrite, WriteArgs, WriteExpression};
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
                        let expr = parse_scalar_list(v)?;

                        args.args.insert(sf, expr);
                    }
                    Field::Scalar(sf) => {
                        let expr: WriteExpression = parse_scalar(sf, v)?;

                        args.args.insert(sf, expr)
                    }

                    Field::Relation(ref rf) => match v {
                        ParsedInputValue::Single(PrismaValue::Null) => (),
                        _ => args.nested.push((Arc::clone(rf), v.try_into()?)),
                    },

                    Field::Composite(cf) => {
                        let expr = parse_composite_writes(cf, v, &mut vec![])?;

                        args.args.insert(cf, expr)
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

fn parse_scalar(sf: &ScalarFieldRef, v: ParsedInputValue) -> Result<WriteExpression, QueryGraphBuilderError> {
    match v {
        ParsedInputValue::Single(PrismaValue::Enum(e)) if sf.type_identifier == TypeIdentifier::Json => {
            let val = match e.as_str() {
                json_null::DB_NULL => PrismaValue::Null,
                json_null::JSON_NULL => PrismaValue::Json("null".to_owned()),
                _ => unreachable!(), // Validation guarantees correct enum values.
            };

            Ok(val.into())
        }
        ParsedInputValue::Single(v) => Ok(v.into()),
        ParsedInputValue::Map(map) => {
            let (operation, value) = map.into_iter().next().unwrap();
            let value: PrismaValue = value.try_into()?;

            let expr = match operation.as_str() {
                operations::SET => WriteExpression::Value(value),
                operations::INCREMENT => WriteExpression::Add(value),
                operations::DECREMENT => WriteExpression::Substract(value),
                operations::MULTIPLY => WriteExpression::Multiply(value),
                operations::DIVIDE => WriteExpression::Divide(value),
                _ => unreachable!("Invalid update operation"),
            };

            Ok(expr)
        }
        _ => unreachable!(),
    }
}

fn parse_scalar_list(v: ParsedInputValue) -> QueryGraphBuilderResult<WriteExpression> {
    match v {
        ParsedInputValue::List(_) => {
            let set_value: PrismaValue = v.try_into()?;

            Ok(WriteExpression::Value(set_value))
        }
        ParsedInputValue::Map(map) => extract_scalar_list_ops(map),
        _ => unreachable!(),
    }
}

fn parse_composite_writes(
    cf: &CompositeFieldRef,
    v: ParsedInputValue,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteExpression> {
    match v {
        // Null-set operation.
        ParsedInputValue::Single(PrismaValue::Null) => Ok(WriteExpression::Value(PrismaValue::Null)),

        // Set list shorthand operation (can only be objects).
        ParsedInputValue::List(_) => {
            let list: PrismaValue = v.try_into()?;

            Ok(WriteExpression::Value(list))
        }

        // One of:
        // - Operation envelope with further actions nested.
        // - Single object set shorthand.
        ParsedInputValue::Map(map) => {
            if is_composite_envelope(&map) {
                parse_composite_envelope(cf, map, path)
            } else {
                Ok(WriteExpression::Value(ParsedInputValue::Map(map).try_into()?))
            }
        }
        _ => unreachable!(),
    }
}

fn parse_composite_envelope(
    cf: &CompositeFieldRef,
    envelope: ParsedInputMap,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteExpression> {
    let (op, value) = envelope.into_iter().next().unwrap();

    let expr = match op.as_str() {
        // Everything in a set operation can only be plain values, no more nested operations.
        operations::SET => WriteExpression::Value(value.try_into()?),
        operations::UPDATE => parse_composite_updates(cf, value.try_into()?, path)?,
        // operations::PUSH => WriteExpression::Add(value.try_into()?),
        _ => unimplemented!(),
    };

    Ok(expr)
}

fn parse_composite_updates(
    cf: &CompositeFieldRef,
    map: ParsedInputMap,
    path: &mut Vec<DatasourceFieldName>,
) -> QueryGraphBuilderResult<WriteExpression> {
    let mut writes = vec![];

    for (k, v) in map {
        let field = cf.typ.find_field(&k).unwrap();

        let field_name: DatasourceFieldName = match field {
            Field::Scalar(sf) => sf.into(),
            Field::Composite(cf) => cf.into(),
            _ => unreachable!(),
        };

        let expr = match field {
            Field::Scalar(sf) if sf.is_list() => parse_scalar_list(v),
            Field::Scalar(sf) => parse_scalar(&sf, v),
            Field::Composite(cf) => parse_composite_writes(&cf, v, &mut path.clone()),
            _ => unreachable!(),
        }?;

        writes.push((field_name, expr));
    }

    Ok(WriteExpression::NestedWrite(NestedWrite { writes }))
}

fn extract_scalar_list_ops(map: ParsedInputMap) -> QueryGraphBuilderResult<WriteExpression> {
    let (operation, value) = map.into_iter().next().unwrap();

    match operation.as_str() {
        operations::SET => Ok(WriteExpression::Value(value.try_into()?)),
        operations::PUSH => Ok(WriteExpression::Add(value.try_into()?)),
        _ => unreachable!("Invalid scalar list operation"),
    }
}
