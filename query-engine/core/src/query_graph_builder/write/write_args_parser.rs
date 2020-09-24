use super::*;
use crate::query_document::{ParsedInputMap, ParsedInputValue};
use connector::{WriteArgs, WriteExpression};
use prisma_models::{Field, ModelRef, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

#[derive(Default, Debug)]
pub struct WriteArgsParser {
    pub args: WriteArgs,
    pub nested: Vec<(RelationFieldRef, ParsedInputMap)>,
}

impl WriteArgsParser {
    /// Creates a new set of WriteArgsParser. Expects the parsed input map from the respective data key, not the enclosing map.
    /// E.g.: { data: { THIS MAP } } from the `data` argument of a write query.
    pub fn from(model: &ModelRef, data_map: ParsedInputMap) -> QueryGraphBuilderResult<Self> {
        data_map.into_iter().try_fold(
            WriteArgsParser::default(),
            |mut args, (k, v): (String, ParsedInputValue)| {
                let field = model.fields().find_from_all(&k).unwrap();

                match field {
                    Field::Scalar(sf) if sf.is_list => {
                        let set_value: PrismaValue = match v {
                            ParsedInputValue::List(_) => v.try_into()?,
                            ParsedInputValue::Map(mut map) => map.remove("set").unwrap().try_into()?,
                            _ => unreachable!(),
                        };

                        args.args.insert(sf, set_value)
                    }

                    Field::Scalar(sf) => {
                        let expr: WriteExpression = match v {
                            ParsedInputValue::Single(v) => v.into(),
                            ParsedInputValue::Map(map) => {
                                let (operation, value) = map.into_iter().next().unwrap();
                                let value: PrismaValue = value.try_into()?;

                                match operation.as_str() {
                                    "set" => WriteExpression::Value(value),
                                    "increment" => WriteExpression::Add(value),
                                    "decrement" => WriteExpression::Substract(value),
                                    "multiply" => WriteExpression::Multiply(value),
                                    "divide" => WriteExpression::Divide(value),
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
