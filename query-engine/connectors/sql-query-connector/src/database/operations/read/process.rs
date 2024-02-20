use std::borrow::Cow;

use itertools::{Either, Itertools};
use query_structure::{QueryArguments, Record};

use crate::query_arguments_ext::QueryArgumentsExt;

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
processor_state!(WithDistinct);

trait ApplyReverseOrder<T>: DoubleEndedIterator<Item = T>
where
    Self: Sized,
{
    fn apply_reverse_order(self, args: &QueryArguments) -> impl DoubleEndedIterator<Item = T> {
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
        mut get_record_and_fields: impl FnMut(&Self::Item) -> Option<(Cow<'_, Record>, Cow<'_, [String]>)> + 'a,
    ) -> impl Iterator<Item = T> {
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
