use std::borrow::Cow;

use crate::{
    context::Context,
    model_extensions::{AsColumn, AsColumns, AsTable},
};

use connector_interface::{QueryArguments, RelAggregationSelection, RelatedQuery};
use itertools::Itertools;
use prisma_models::ModelProjection;
use quaint::{prelude::*, visitor::*};

/*

SELECT
    "Link"."id",
  "Link"."createdAt",
  "Link"."updatedAt",
  "Link"."url",
  "Link"."shortUrl",
  "Link"."userId",
  "A"."json"
FROM "Link"
    LEFT JOIN LATERAL (
    SELECT JSON_AGG("json") AS "json" FROM (
      SELECT JSON_BUILD_OBJECT('linkId', "LinkOpen"."linkId", 'createdAt', "LinkOpen"."createdAt") AS "json"
      FROM "LinkOpen"
      WHERE "LinkOpen"."linkId" = "Link"."id"
      ORDER BY "LinkOpen"."createdAt"
      LIMIT 1
    ) "A"
  ) "A" ON true
LIMIT 10;
*/
pub(crate) fn build(
    args: QueryArguments,
    nested: Vec<RelatedQuery>,
    selection: &ModelProjection,
    _aggr_selections: &[RelAggregationSelection],
    ctx: &Context<'_>,
) {
    dbg!(&selection);
    dbg!(&nested);
    let select = Select::from_table(args.model().as_table(ctx));

    // scalars selection
    let select = selection.fields().fold(select, |acc, selection| match selection {
        prisma_models::Field::Relation(rf) => acc.value(rf.name().to_owned()),
        prisma_models::Field::Scalar(sf) => acc.column(sf.as_column(ctx)),
        prisma_models::Field::Composite(_) => unreachable!(),
    });

    // TODO: check how to select aggregated relations
    let select = nested
        .iter()
        .fold(select, |acc, read| acc.value(Column::from(read.name.to_owned())));

    let select = nested.into_iter().fold(select, |acc, nested| {
      let join_select = build_nested(nested, ctx);
      let join_table = Table::from(nested.parent_field.model().as_table(ctx)).left_join(select);

    })

    for n in nested {
        let select = build_nested(n, ctx);
    }

    let (sql, _) = Postgres::build(select).unwrap();

    dbg!(&sql);
}

pub(crate) fn build_nested(nested: RelatedQuery, ctx: &Context<'_>) -> Select<'static> {
    /*
    ```sql
      SELECT
          "Link"."id",
        "Link"."createdAt",
        "Link"."updatedAt",
        "Link"."url",
        "Link"."shortUrl",
        "Link"."userId",
        "A"."json"
      FROM "Link"
          LEFT JOIN LATERAL (
          SELECT JSON_AGG("json") AS "json" FROM ( -- inner
            SELECT JSON_BUILD_OBJECT('linkId', "LinkOpen"."linkId", 'createdAt', "LinkOpen"."createdAt") AS "json"
            FROM "LinkOpen"
            WHERE "LinkOpen"."linkId" = "Link"."id"
            ORDER BY "LinkOpen"."createdAt"
            LIMIT 1
          ) "A"
        ) "A" ON true
      LIMIT 10;
    ```
      */
    let build_obj_params = nested
        .selected_fields
        .into_iter()
        .map(|f| match f {
            prisma_models::SelectedField::Scalar(sf) => {
                (Cow::from(sf.name().to_owned()), Expression::from(sf.as_column(ctx)))
            }
            _ => unreachable!(),
        })
        .collect_vec();
    let inner = Select::from_table(nested.parent_field.model().as_table(ctx))
        .value(json_build_object(build_obj_params).alias("json"));

    let select = Select::from(inner).value(json_array_agg(Column::from("json")));

    select
}
