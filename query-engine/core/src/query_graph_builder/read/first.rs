use super::*;

pub struct ReadFirstRecordBuilder(pub ReadManyRecordsBuilder);

impl Builder<ReadQuery> for ReadFirstRecordBuilder {
    fn build(self) -> QueryGraphBuilderResult<ReadQuery> {
        let mut many_query = self.0.build()?;

        // Optimization: Add `take: 1` to the query to reduce fetched result set size if possible.
        Ok(match many_query {
            ReadQuery::ManyRecordsQuery(ref mut m) if m.args.take.is_none() => {
                m.args.take = Some(1);
                many_query
            }
            _ => many_query,
        })
    }
}
