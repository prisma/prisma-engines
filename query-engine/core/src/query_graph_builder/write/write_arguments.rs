use super::*;
use crate::query_document::{ParsedInputMap, ParsedInputValue};
use prisma_models::{Field, ModelRef, PrismaArgs, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

#[derive(Default, Debug)]
pub struct WriteArguments {
    pub args: PrismaArgs,
    pub nested: Vec<(RelationFieldRef, ParsedInputMap)>,
}

impl WriteArguments {
    /// Creates a new set of WriteArguments. Expects the parsed input map from the respective data key, not the enclosing map.
    /// E.g.: { data: { THIS MAP } } from the `data` argument of a write query.
    pub fn from(model: &ModelRef, data_map: ParsedInputMap) -> QueryGraphBuilderResult<Self> {
        data_map.into_iter().try_fold(
            WriteArguments::default(),
            |mut args, (k, v): (String, ParsedInputValue)| {
                let field = model.fields().find_from_all(&k).unwrap();
                match field {
                    Field::Scalar(sf) if sf.is_list => {
                        let vals: ParsedInputMap = v.try_into()?;
                        let set_value: PrismaValue =
                            vals.into_iter().find(|(k, _)| k == "set").unwrap().1.try_into()?;

                        args.args.insert(sf.name.clone(), set_value)
                    }

                    Field::Scalar(sf) => {
                        let value: PrismaValue = v.try_into()?;
                        args.args.insert(sf.name.clone(), value)
                    }

                    Field::Relation(ref rf) => {
                        args.nested.push((Arc::clone(rf), v.try_into()?));
                    }
                };

                Ok(args)
            },
        )
    }
}
