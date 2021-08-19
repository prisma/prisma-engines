use connector::Filter;

pub fn flatten_filter(filter: Filter) -> Filter {
    fn flatten_filters(filters: Vec<Filter>, parent: &Filter) -> Vec<Filter> {
        let mut flattened: Vec<Filter> = vec![];

        for f in filters {
            match (f.clone(), parent) {
                (Filter::And(and), Filter::And(_)) => {
                    flattened.append(&mut flatten_filters(and, &f));
                }
                (Filter::And(and), _) => {
                    flattened.push(Filter::And(flatten_filters(and, &f)));
                }
                (Filter::Or(or), Filter::Or(_)) => {
                    flattened.append(&mut flatten_filters(or, &f));
                }
                (Filter::Or(or), _) => {
                    flattened.push(Filter::Or(flatten_filters(or, &f)));
                }
                (Filter::Not(not), Filter::Not(_)) => {
                    flattened.append(&mut flatten_filters(not, &f));
                }
                (Filter::Not(not), _) => {
                    flattened.push(Filter::Not(flatten_filters(not, &f)));
                }
                _ => {
                    flattened.push(f);
                }
            }
        }

        flattened
    }

    let parent = filter.clone();

    match filter {
        Filter::And(ands) => Filter::And(flatten_filters(ands, &parent)),
        Filter::Or(ors) => Filter::Or(flatten_filters(ors, &parent)),
        Filter::Not(nots) => Filter::Not(flatten_filters(nots, &parent)),
        f => f,
    }
}
