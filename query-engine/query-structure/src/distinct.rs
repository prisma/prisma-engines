use crate::{FieldSelection, OrderBy};

/// Checks that the ordering is compatible with native DISTINCT ON in connectors that support it.
///
/// If order by is present, distinct fields must match leftmost order by fields in the query. The
/// order of the distinct fields does not necessarily have to be the same as the order of the
/// corresponding fields in the leftmost subset of `order_by` but the distinct fields must come
/// before non-distinct fields in the order by clause. Order by clause may contain only a subset of
/// the distinct fields if no other fields are being used for ordering.
///
/// If there's no order by, then DISTINCT ON is allowed for any fields.
pub fn native_distinct_compatible_with_order_by(
    distinct_fields: Option<&FieldSelection>,
    order_by_fields: &[OrderBy],
) -> bool {
    if order_by_fields.is_empty() {
        return true;
    }

    let Some(distinct_fields) = distinct_fields else {
        return true;
    };

    let count_leftmost_matching = order_by_fields
        .iter()
        .take_while(|order_by| match order_by {
            OrderBy::Scalar(scalar) if scalar.path.is_empty() => {
                distinct_fields.scalars().any(|sf| *sf == scalar.field)
            }
            _ => false,
        })
        .count();

    count_leftmost_matching == usize::min(distinct_fields.as_ref().len(), order_by_fields.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use crate::{ScalarFieldRef, native_distinct_compatible_with_order_by};

    struct TestFields {
        a: ScalarFieldRef,
        b: ScalarFieldRef,
        c: ScalarFieldRef,
    }

    impl TestFields {
        fn new() -> Self {
            let schema_str = r#"
                datasource db {
                    provider = "postgresql"
                    url = "postgres://stub"
                }

                model Test {
                    id Int @id
                    a  Int
                    b  Int
                    c  Int
                }
            "#;

            let psl_schema = psl::validate(schema_str.into());
            let internal_datamodel = crate::InternalDataModel {
                schema: Arc::new(psl_schema),
            };

            let model = internal_datamodel.find_model("Test").unwrap();
            let fields = model.fields();

            TestFields {
                a: fields.find_from_scalar("a").unwrap(),
                b: fields.find_from_scalar("b").unwrap(),
                c: fields.find_from_scalar("c").unwrap(),
            }
        }
    }

    mod native_distinct_compatible_with_order_by {
        use super::*;

        #[test]
        fn empty_order_by() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a]);
            let order_by = [];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn empty_distinct() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([]);
            let order_by = [OrderBy::from(fields.a)];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
            assert!(native_distinct_compatible_with_order_by(None, &order_by));
        }

        #[test]
        fn exact_match() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone()]);
            let order_by = [OrderBy::from(fields.a)];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn exact_match_mixed_order() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone(), fields.b.clone()]);
            let order_by = [OrderBy::from(fields.b), OrderBy::from(fields.a)];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn left_subset() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone()]);
            let order_by = [OrderBy::from(fields.a), OrderBy::from(fields.b)];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn left_subset_mixed_order() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone(), fields.b.clone()]);
            let order_by = [
                OrderBy::from(fields.b),
                OrderBy::from(fields.a),
                OrderBy::from(fields.c),
            ];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn incompatible_left_field() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone(), fields.b.clone()]);
            let order_by = [
                OrderBy::from(fields.c),
                OrderBy::from(fields.a),
                OrderBy::from(fields.b),
            ];

            assert!(!native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn incompatible_field_in_between() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone(), fields.b.clone()]);
            let order_by = [
                OrderBy::from(fields.a),
                OrderBy::from(fields.c),
                OrderBy::from(fields.b),
            ];

            assert!(!native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn partial_order_first() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone(), fields.b.clone()]);
            let order_by = [OrderBy::from(fields.a)];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn partial_order_second() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone(), fields.b.clone()]);
            let order_by = [OrderBy::from(fields.b)];

            assert!(native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }

        #[test]
        fn incompatible_partial_order() {
            let fields = TestFields::new();

            let distinct = FieldSelection::from([fields.a.clone(), fields.b.clone()]);
            let order_by = [OrderBy::from(fields.c)];

            assert!(!native_distinct_compatible_with_order_by(Some(&distinct), &order_by));
        }
    }
}
