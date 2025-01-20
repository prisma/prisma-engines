use std::borrow::Cow;

use itertools::{Either, Itertools};
use query_builder::QueryArgumentsExt;
use query_structure::{QueryArguments, Record};

macro_rules! processor_state {
    ($name:ident $(-> $transition:ident($bound:ident))?) => {
        struct $name<T>(T);

        impl<T, U> Iterator for $name<T>
        where
            T: Iterator<Item = U>,
        {
            type Item = U;

            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }
        }

        impl<T, U> DoubleEndedIterator for $name<T>
        where
            T: DoubleEndedIterator<Item = U>,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.0.next_back()
            }
        }

        $(
            impl<T, U> $transition<T> for $name<U> where U: $bound<Item = T> {}
        )?
    };
}

processor_state!(Initial -> ApplyReverseOrder(DoubleEndedIterator));
processor_state!(WithReverseOrder -> ApplyDistinct(Iterator));
processor_state!(WithDistinct -> ApplyPagination(Iterator));
processor_state!(WithPagination);

trait ApplyReverseOrder<T>: DoubleEndedIterator<Item = T>
where
    Self: Sized,
{
    fn apply_reverse_order(self, args: &QueryArguments) -> WithReverseOrder<impl DoubleEndedIterator<Item = T>> {
        WithReverseOrder(match args.needs_reversed_order() {
            true => Either::Left(self.rev()),
            false => Either::Right(self),
        })
    }
}

trait ApplyDistinct<T>: Iterator<Item = T>
where
    Self: Sized,
{
    fn apply_distinct<'a>(
        self,
        args: &'a QueryArguments,
        mut get_record_and_fields: impl for<'b> FnMut(&'b Self::Item) -> Option<(Cow<'b, Record>, Cow<'a, [String]>)> + 'a,
    ) -> WithDistinct<impl Iterator<Item = T>> {
        WithDistinct(match args.distinct.as_ref() {
            Some(distinct) if args.requires_inmemory_distinct_with_joins() => {
                Either::Left(self.unique_by(move |value| {
                    get_record_and_fields(value).map(|(record, field_names)| {
                        record
                            .extract_selection_result_from_prisma_name(&field_names, distinct)
                            .unwrap()
                    })
                }))
            }
            _ => Either::Right(self),
        })
    }
}

trait ApplyPagination<T>: Iterator<Item = T>
where
    Self: Sized,
{
    fn apply_pagination(self, args: &QueryArguments) -> WithPagination<impl Iterator<Item = T>> {
        let iter = match args.skip {
            Some(skip) if args.requires_inmemory_pagination_with_joins() => Either::Left(self.skip(skip as usize)),
            _ => Either::Right(self),
        };

        let iter = match args.take_abs() {
            Some(take) if args.requires_inmemory_pagination_with_joins() => Either::Left(iter.take(take as usize)),
            _ => Either::Right(iter),
        };

        WithPagination(iter)
    }
}

pub struct InMemoryProcessorForJoins<'a, I> {
    args: &'a QueryArguments,
    records: I,
}

impl<'a, T, I> InMemoryProcessorForJoins<'a, I>
where
    T: 'a,
    I: DoubleEndedIterator<Item = T> + 'a,
{
    pub fn new(args: &'a QueryArguments, records: impl IntoIterator<IntoIter = I>) -> Self {
        Self {
            args,
            records: records.into_iter(),
        }
    }

    pub fn process(
        self,
        get_record_and_fields: impl for<'b> FnMut(&'b T) -> Option<(Cow<'b, Record>, Cow<'a, [String]>)> + 'a,
    ) -> impl Iterator<Item = T> + 'a {
        Initial(self.records)
            .apply_reverse_order(self.args)
            .apply_distinct(self.args, get_record_and_fields)
            .apply_pagination(self.args)
    }
}
