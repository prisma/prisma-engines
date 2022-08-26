use crate::{query_graph_builder::extractors::utils, *};

use connector::{UniqueFilter, UniqueFilters};
use prisma_models::*;

pub fn extract_unique_filters(
    mut input_map: ParsedInputMap,
    model: &ModelRef,
) -> QueryGraphBuilderResult<UniqueFilters> {
    let mut unique_filters: UniqueFilters = UniqueFilters::default();

    for (field_name, map_value) in input_map.clone() {
        match model.fields().find_from_all(&field_name) {
            Ok(Field::Scalar(sf)) => {
                let mut pv: PrismaValue = map_value.try_into()?;

                unique_filters.add_filter(UniqueFilter::scalar(sf.clone(), pv));
            }
            Ok(Field::Composite(cf)) => {
                let unique_index = cf.unique_index().as_ref().unwrap();
                let filters = extract_index_filters(unique_index.fields(), &mut input_map)?;

                unique_filters.add_filters(filters);
            }
            Ok(Field::Relation(_)) => unreachable!(),
            Err(_) => match utils::resolve_compound_field(&field_name, &model) {
                Some(index_fields) => {
                    let mut map: ParsedInputMap = map_value.try_into()?;
                    let cursor_filters = extract_index_filters(&index_fields, &mut map)?;

                    unique_filters.add_filters(cursor_filters);
                }
                None => {
                    return Err(QueryGraphBuilderError::AssertionError(format!(
                        "Unable to resolve field {} to a field or a set of fields on model {}",
                        field_name, model.name
                    )))
                }
            },
        }
    }

    Ok(unique_filters)
}

fn extract_index_filters(
    index_fields: &[IndexField],
    map: &mut ParsedInputMap,
) -> QueryGraphBuilderResult<Vec<UniqueFilter>> {
    let mut filters: Vec<UniqueFilter> = vec![];

    for index_field in index_fields {
        match index_field {
            IndexField::Scalar(sf) => {
                let pv: PrismaValue = map.remove(&sf.name).unwrap().try_into()?;

                filters.push(UniqueFilter::scalar(sf.clone(), pv));
            }
            IndexField::Composite(cif) if cif.is_partial() => {
                let mut inner_map: ParsedInputMap = map.remove(&cif.field().name).unwrap().try_into()?;
                let nested = extract_index_filters(&cif.index_fields(), &mut inner_map)?;

                filters.push(UniqueFilter::composite_partial(cif.field().clone(), nested));
            }
            IndexField::Composite(cif) if cif.field().is_list() => {
                let inner_value = map.remove(&cif.field().name).unwrap();
                let pv: PrismaValue = inner_value.clone().try_into()?;

                filters.push(UniqueFilter::composite(cif.field().clone(), pv, vec![]));
            }
            IndexField::Composite(cif) => {
                let inner_value = map.remove(&cif.field().name).unwrap();

                let pv: PrismaValue = inner_value.clone().try_into()?;
                let mut inner_map: ParsedInputMap = inner_value.try_into()?;

                let nested = extract_index_filters(&cif.index_fields(), &mut inner_map)?;

                filters.push(UniqueFilter::composite(cif.field().clone(), pv, nested));
            }
        }
    }

    Ok(filters)
}
