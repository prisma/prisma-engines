use query_structure::Filter;

pub fn fold_filter(filter: Filter) -> Filter {
    match filter {
        Filter::And(and) => fold_and(and),
        Filter::Or(or) => fold_or(or),
        Filter::Not(not) => fold_not(not),
        f => f,
    }
}

fn fold_and(filters: Vec<Filter>) -> Filter {
    fn fold_and_impl(filters: Vec<Filter>) -> Vec<Filter> {
        let mut flattened: Vec<Filter> = vec![];

        // Parent is AND([...])
        for f in filters {
            match f.clone() {
                Filter::And(and) => flattened.append(&mut fold_and_impl(and)),
                Filter::Or(or) => flattened.push(fold_or(or)),
                Filter::Not(not) => flattened.push(fold_not(not)),
                filter => flattened.push(filter),
            }
        }
        flattened
    }

    let folded = fold_and_impl(filters);

    if folded.len() == 1 {
        folded.first().unwrap().clone()
    } else {
        Filter::And(folded)
    }
}

fn fold_or(filters: Vec<Filter>) -> Filter {
    fn fold_or_impl(filters: Vec<Filter>) -> Vec<Filter> {
        let mut flattened: Vec<Filter> = vec![];

        // Parent is OR([...])
        for f in filters {
            match f.clone() {
                Filter::Or(or) => flattened.append(&mut fold_or_impl(or)),
                Filter::And(and) => flattened.push(fold_and(and)),
                Filter::Not(not) => flattened.push(fold_not(not)),
                filter => flattened.push(filter),
            }
        }
        flattened
    }

    let folded = fold_or_impl(filters);

    if folded.len() == 1 {
        folded.first().unwrap().clone()
    } else {
        Filter::Or(folded)
    }
}

fn fold_not(filters: Vec<Filter>) -> Filter {
    fn fold_not_impl(filters: Vec<Filter>) -> Vec<Filter> {
        let mut res: Vec<Filter> = vec![];

        for f in filters {
            match f.clone() {
                Filter::Not(not) => res.push(fold_not(not)),
                Filter::And(and) => res.push(fold_and(and)),
                Filter::Or(or) => res.push(fold_or(or)),
                filter => {
                    res.push(filter);
                }
            }
        }

        res
    }

    let folded = fold_not_impl(filters);

    Filter::Not(folded)
}

#[test]
fn ensure_and_folded() {
    let input = fold_filter(Filter::And(vec![Filter::Empty, Filter::And(vec![Filter::Empty])]));
    let expected_output = Filter::And(vec![Filter::Empty, Filter::Empty]);

    assert_eq!(input, expected_output)
}

#[test]
fn ensure_or_folded() {
    let input = fold_filter(Filter::Or(vec![Filter::Empty, Filter::Or(vec![Filter::Empty])]));
    let expected_output = Filter::Or(vec![Filter::Empty, Filter::Empty]);

    assert_eq!(input, expected_output)
}

#[test]
fn ensure_not_is_not_folded() {
    let input = fold_filter(Filter::Not(vec![Filter::Empty, Filter::Not(vec![Filter::Empty])]));
    let expected_output = Filter::Not(vec![Filter::Empty, Filter::Not(vec![Filter::Empty])]);
    assert_eq!(input, expected_output);

    let input = fold_filter(Filter::Not(vec![
        Filter::Empty,
        Filter::Not(vec![Filter::Not(vec![Filter::Empty])]),
    ]));
    // TODO: `Not(Not())` could be folded
    let expected_output = Filter::Not(vec![Filter::Empty, Filter::Not(vec![Filter::Not(vec![Filter::Empty])])]);
    assert_eq!(input, expected_output);
}

#[test]
fn ensure_nested_conditions_are_folded() {
    let input = fold_filter(Filter::Not(vec![
        Filter::Empty,
        Filter::Not(vec![Filter::Not(vec![Filter::Empty])]),
        Filter::Or(vec![
            Filter::And(vec![Filter::Empty, Filter::And(vec![Filter::Empty, Filter::Empty])]),
            Filter::Or(vec![Filter::Empty, Filter::Or(vec![Filter::Empty, Filter::Empty])]),
        ]),
    ]));
    let expected_output = Filter::Not(vec![
        Filter::Empty,
        Filter::Not(vec![Filter::Not(vec![Filter::Empty])]),
        Filter::Or(vec![
            Filter::And(vec![Filter::Empty, Filter::Empty, Filter::Empty]),
            Filter::Empty,
            Filter::Empty,
            Filter::Empty,
        ]),
    ]);

    assert_eq!(input, expected_output)
}

#[test]
fn ensure_filter_fold_does_not_alter_boolean_logic() {
    let filters = generate_filters();

    for filter in filters {
        let folded_filter = fold_filter(filter.clone());
        let visitor = FilterVisitor::new(filter.clone());
        let folded_visitor = FilterVisitor::new(folded_filter.clone());
        let visited_filter = visitor.visit();
        let visited_folded_filter = folded_visitor.visit();

        if visited_filter != visited_folded_filter {
            dbg!(&filter);
            dbg!(&folded_filter);
        }

        assert_eq!(visitor.visit(), folded_visitor.visit())
    }
}

#[cfg(test)]
struct FilterVisitor {
    filter: Filter,
}

#[cfg(test)]
impl FilterVisitor {
    pub fn new(filter: Filter) -> Self {
        Self { filter }
    }

    pub fn visit(&self) -> bool {
        match &self.filter {
            Filter::And(and) => self.visit_and(and),
            Filter::Or(or) => self.visit_or(or),
            Filter::Not(not) => self.visit_not(not),
            Filter::BoolFilter(b) => *b,
            _ => unreachable!(),
        }
    }

    fn visit_and(&self, filters: &[Filter]) -> bool {
        let mut res = true;

        for (index, f) in filters.iter().enumerate() {
            match f {
                Filter::And(and) if index == 0 => {
                    res = self.visit_and(and);
                }
                Filter::And(and) => {
                    res = res && self.visit_and(and);
                }
                Filter::Or(or) if index == 0 => {
                    res = self.visit_or(or);
                }
                Filter::Or(or) => {
                    res = res && self.visit_or(or);
                }
                Filter::Not(not) if index == 0 => {
                    res = self.visit_not(not);
                }
                Filter::Not(not) => {
                    res = res && self.visit_not(not);
                }
                Filter::BoolFilter(b) if index == 0 => {
                    res = *b;
                }
                Filter::BoolFilter(b) => {
                    res = res && *b;
                }
                _ => unreachable!(),
            }
        }

        res
    }

    fn visit_or(&self, filters: &[Filter]) -> bool {
        let mut res = true;

        for (index, f) in filters.iter().enumerate() {
            match f {
                Filter::And(and) if index == 0 => {
                    res = self.visit_and(and);
                }
                Filter::And(and) => {
                    res = res || self.visit_and(and);
                }
                Filter::Or(or) if index == 0 => {
                    res = self.visit_or(or);
                }
                Filter::Or(or) => {
                    res = res || self.visit_or(or);
                }
                Filter::Not(not) if index == 0 => {
                    res = self.visit_not(not);
                }
                Filter::Not(not) => {
                    res = res || self.visit_not(not);
                }
                Filter::BoolFilter(b) if index == 0 => {
                    res = *b;
                }
                Filter::BoolFilter(b) => {
                    res = res || *b;
                }
                _ => unreachable!(),
            }
        }

        res
    }

    fn visit_not(&self, filters: &[Filter]) -> bool {
        let mut res = true;

        for (index, f) in filters.iter().enumerate() {
            match f {
                Filter::And(and) if index == 0 => {
                    res = !self.visit_and(and);
                }
                Filter::And(and) => {
                    res = res && !self.visit_and(and);
                }
                Filter::Or(or) if index == 0 => {
                    res = !self.visit_or(or);
                }
                Filter::Or(or) => {
                    res = res && !self.visit_or(or);
                }
                Filter::Not(not) if index == 0 => {
                    res = !self.visit_not(not);
                }
                Filter::Not(not) => {
                    res = res && !self.visit_not(not);
                }
                Filter::BoolFilter(b) if index == 0 => {
                    res = !*b;
                }
                Filter::BoolFilter(b) => {
                    res = res && !*b;
                }
                _ => unreachable!(),
            }
        }

        res
    }
}

// This helper deterministically generates a bunch of filters.
// eg: And([true]), And([true, false]), ... Or([true]), Or([true, false]), ... And([true, Or([true, false])]) etc
#[cfg(test)]
fn generate_filters() -> Vec<Filter> {
    let all_vars = combinations(vec![Filter::BoolFilter(true), Filter::BoolFilter(false)]);
    let all_conditions = combinations(vec![Filter::And(vec![]), Filter::Or(vec![]), Filter::Not(vec![])]);
    let mut filters: Vec<Filter> = vec![];

    for conditions in all_conditions {
        for vars in all_vars.clone() {
            let mut conditions_with_vars: Vec<Filter> = vec![];

            for cond in conditions.clone().iter_mut() {
                match cond {
                    Filter::And(f) => {
                        f.append(&mut vars.clone());
                    }
                    Filter::Or(f) => {
                        f.append(&mut vars.clone());
                    }
                    Filter::Not(f) => {
                        f.append(&mut vars.clone());
                    }
                    _ => unreachable!(),
                }

                conditions_with_vars.push(cond.clone());
            }

            let (nested_conditions, rest) = conditions_with_vars.split_first_mut().unwrap();

            match nested_conditions {
                Filter::And(f) => f.append(&mut rest.to_vec()),
                Filter::Or(f) => f.append(&mut rest.to_vec()),
                Filter::Not(f) => f.append(&mut rest.to_vec()),
                _ => unreachable!(),
            }

            filters.push(nested_conditions.clone());
        }
    }
    filters
}

#[cfg(test)]
fn combinations<T: Clone>(vec: Vec<T>) -> Vec<Vec<T>> {
    let mut output: Vec<Vec<T>> = vec![];

    for a in &vec {
        output.push(vec![a.clone()]);
        for b in &vec {
            output.push(vec![a.clone(), b.clone()]);
            for c in &vec {
                output.push(vec![a.clone(), b.clone(), c.clone()]);
            }
        }
    }

    output
}
