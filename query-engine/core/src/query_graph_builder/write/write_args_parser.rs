use super::*;
use crate::{
    constants::{json_null, operations},
    query_document::{ParsedInputMap, ParsedInputValue},
};
use connector::{WriteArgs, WriteExpression};
use prisma_models::{Field, ModelRef, PrismaValue, RelationFieldRef, TypeIdentifier};
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
                    Field::Scalar(sf) if sf.is_list => match v {
                        ParsedInputValue::List(_) => {
                            let set_value: PrismaValue = v.try_into()?;

                            args.args.insert(sf, set_value);
                        }
                        ParsedInputValue::Map(map) => {
                            let expr = extract_scalar_list_ops(map)?;

                            args.args.insert(sf, expr)
                        }
                        _ => unreachable!(),
                    },

                    Field::Scalar(sf) => {
                        let expr: WriteExpression = match v {
                            ParsedInputValue::Single(PrismaValue::Enum(e))
                                if sf.type_identifier == TypeIdentifier::Json =>
                            {
                                let val = match e.as_str() {
                                    json_null::DB_NULL => PrismaValue::Null,
                                    json_null::JSON_NULL => PrismaValue::Json("null".to_owned()),
                                    _ => unreachable!(), // Validation guarantees correct enum values.
                                };

                                val.into()
                            }
                            ParsedInputValue::Single(v) => v.into(),
                            ParsedInputValue::Map(map) => {
                                let (operation, value) = map.into_iter().next().unwrap();
                                let value: PrismaValue = value.try_into()?;

                                match operation.as_str() {
                                    operations::SET => WriteExpression::Value(value),
                                    operations::INCREMENT => WriteExpression::Add(value),
                                    operations::DECREMENT => WriteExpression::Substract(value),
                                    operations::MULTIPLY => WriteExpression::Multiply(value),
                                    operations::DIVIDE => WriteExpression::Divide(value),
                                    _ => unreachable!("Invalid update operation"),
                                }
                            }
                            _ => unreachable!(),
                        };

                        args.args.insert(sf, expr)
                    }

                    Field::Relation(ref rf) => match v {
                        ParsedInputValue::Single(PrismaValue::Null) => (),
                        _ => args.nested.push((Arc::clone(rf), v.try_into()?)),
                    },
                };

                Ok(args)
            },
        )
    }
}

fn extract_scalar_list_ops(map: ParsedInputMap) -> QueryGraphBuilderResult<WriteExpression> {
    let (operation, value) = map.into_iter().next().unwrap();

    match operation.as_str() {
        operations::SET => Ok(WriteExpression::Value(value.try_into()?)),
        operations::PUSH => Ok(WriteExpression::Add(value.try_into()?)),
        _ => unreachable!("Invalid scalar list operation"),
    }
}
