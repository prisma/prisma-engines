//! This module is responsible for defining the relation names and the relation field names in an
//! introspected schema with as much clarity and as little ambiguity as possible.

use crate::{datamodel_calculator::InputContext, introspection_helpers::is_prisma_join_table};
use sql_schema_describer as sql;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

/// [relation name, model A relation field name, model B relation field name]
pub(crate) type RelationName<'a> = [Cow<'a, str>; 3];

#[derive(Default)]
pub(crate) struct RelationNames<'a> {
    inline_relation_names: HashMap<sql::ForeignKeyId, RelationName<'a>>,
    m2m_relation_names: HashMap<sql::TableId, RelationName<'a>>,
}

impl<'a> RelationNames<'a> {
    pub(crate) fn inline_relation_name(&self, id: sql::ForeignKeyId) -> Option<&RelationName<'a>> {
        self.inline_relation_names.get(&id)
    }

    #[track_caller]
    pub(crate) fn m2m_relation_name(&self, id: sql::TableId) -> &RelationName<'a> {
        &self.m2m_relation_names[&id]
    }
}

/// This function is responsible for inferring names for the relations and their relation fields in
/// the introspected schema. It does not take into account relations from the existing Prisma
/// schema.
///
/// When we talk about relation names, this is the part we mean:
///
///
/// ```ignore
/// model Test {
///     other Other @relation("TestToOther", map: "the_fk_name")
/// //                        ^^^^^^^^^^^^^
/// }
/// ```
///
/// The name of the relation field in this case is `other`.
///
/// Relation names are mostly useful for the purpose of distinguishing relations from each other
/// when there are multiple relations between the same models.
///
/// Here, we define an _ambiguous relation_ as any relation between two models that coexists with
/// another relation between the same two models.
///
/// The naming rules are the following:
///
/// - Relations inferred from Prisma many-to-many join tables take the name of the table as a
///   relation name, with the prefix underscore removed. In case of ambiguity, the relation field
///   names are disambiguating by appending the relation name.
/// - Inline relations (backed by a foreign key) take a name of the form
///   "$REFERENCING_MODEL_${REFERENCING_FIELD_NAMES}_${MODEL_A_NAME}To${MODEL_B_NAME}"
///   and their relation fields one of the form
///   $OPPOSITE_MODEL_NAME_$CONSTRAINED_FIELD_NAMES_${MODEL_A_NAME}To${MODEL_B_NAME}"
///
/// Additionally, in self-relations, the names of the two relation fields are disambiguated by
/// prefixing the name of the backrelation field with "other_".
pub(super) fn introspect<'a>(input: InputContext<'a>, map: &mut super::IntrospectionMap<'a>) {
    let mut names = RelationNames {
        inline_relation_names: Default::default(),
        m2m_relation_names: Default::default(),
    };

    let mut duplicated_fks = Default::default();
    let ambiguous_relations = find_ambiguous_relations(input);

    for table in input.schema.table_walkers() {
        if is_prisma_join_table(table) {
            let name = prisma_m2m_relation_name(table, &ambiguous_relations, input);
            names.m2m_relation_names.insert(table.id, name);
        } else {
            collect_duplicated_fks(table, &mut duplicated_fks);
            for fk in table.foreign_keys().filter(|fk| !duplicated_fks.contains(&fk.id)) {
                names
                    .inline_relation_names
                    .insert(fk.id, inline_relation_name(fk, &ambiguous_relations, input));
            }
        }
    }

    map.relation_names = names;
}

fn prisma_m2m_relation_name<'a>(
    table: sql::TableWalker<'a>,
    ambiguous_relations: &HashSet<[sql::TableId; 2]>,
    input: InputContext,
) -> RelationName<'a> {
    let ids = table_ids_for_m2m_relation_table(table);
    let is_self_relation = ids[0] == ids[1];

    let (relation_name, field_name_suffix) = if ambiguous_relations.contains(&ids) {
        // the table names of prisma m2m tables starts with an underscore
        (Cow::Borrowed(&table.name()[1..]), table.name())
    } else {
        let default_name = ids.map(|id| input.table_prisma_name(id).prisma_name()).join("To");
        let found_name = &table.name()[1..];
        let relation_name = if found_name == default_name && !is_self_relation {
            ""
        } else {
            found_name
        };
        (Cow::Borrowed(relation_name), "")
    };

    [
        relation_name,
        Cow::Owned(format!(
            "{}{field_name_suffix}{}",
            input.table_prisma_name(ids[1]).prisma_name(),
            if is_self_relation { "_A" } else { "" },
        )),
        Cow::Owned(format!(
            "{}{field_name_suffix}{}",
            input.table_prisma_name(ids[0]).prisma_name(),
            if is_self_relation { "_B" } else { "" },
        )),
    ]
}

fn inline_relation_name<'a>(
    fk: sql::ForeignKeyWalker<'a>,
    ambiguous_relations: &HashSet<[sql::TableId; 2]>,
    input: InputContext<'a>,
) -> RelationName<'a> {
    let is_self_relation = fk.is_self_relation();
    let referencing_model_name = input.table_prisma_name(fk.table().id).prisma_name();
    let referenced_model_name = input.table_prisma_name(fk.referenced_table().id).prisma_name();
    let self_relation_prefix = if is_self_relation { "other_" } else { "" };

    let is_ambiguous_name = ambiguous_relations.contains(&sorted_table_ids(fk.table().id, fk.referenced_table().id));

    if !is_ambiguous_name {
        let relation_name = if is_self_relation {
            Cow::Owned(format!("{referencing_model_name}To{referenced_model_name}"))
        } else {
            Cow::Borrowed("")
        };
        [
            relation_name,
            referenced_model_name,
            Cow::Owned(format!("{self_relation_prefix}{referencing_model_name}")),
        ]
    } else {
        let mut relation_name = referencing_model_name.clone().into_owned();
        relation_name.push('_');
        let mut cols = fk.constrained_columns().peekable();
        while let Some(col) = cols.next() {
            relation_name.push_str(input.column_prisma_name(col.id).prisma_name().as_ref());
            if cols.peek().is_some() {
                relation_name.push('_');
            }
        }
        relation_name.push_str("To");
        relation_name.push_str(&referenced_model_name);

        let forward = format!("{referenced_model_name}_{relation_name}");
        let back = format!("{self_relation_prefix}{referencing_model_name}_{relation_name}");

        [Cow::Owned(relation_name), Cow::Owned(forward), Cow::Owned(back)]
    }
}

/// Relation names are only ambiguous between two given models.
fn find_ambiguous_relations(input: InputContext) -> HashSet<[sql::TableId; 2]> {
    let mut ambiguous_relations = HashSet::new();

    for table in input.schema.table_walkers() {
        if is_prisma_join_table(table) {
            m2m_relation_ambiguousness(table, &mut ambiguous_relations)
        } else {
            for fk in table.foreign_keys() {
                inline_relation_ambiguousness(fk, &mut ambiguous_relations, input)
            }
        }
    }

    ambiguous_relations
}

fn m2m_relation_ambiguousness(table: sql::TableWalker<'_>, ambiguous_relations: &mut HashSet<[sql::TableId; 2]>) {
    let tables = table_ids_for_m2m_relation_table(table);

    if ambiguous_relations.contains(&tables) {
        return;
    }

    // Check for conflicts with an inline relation.
    for model_table in tables {
        for fk in table.walk(model_table).foreign_keys() {
            let fk_tables = sorted_table_ids(model_table, fk.referenced_table().id);
            if fk_tables == tables {
                ambiguous_relations.insert(tables);
            }
        }
    }

    // Check for conflicts with another m2m relation.
    for other_m2m in table.schema.table_walkers().filter(|t| is_prisma_join_table(*t)) {
        if other_m2m.id != table.id && table_ids_for_m2m_relation_table(other_m2m) == tables {
            ambiguous_relations.insert(tables);
        }
    }
}

fn inline_relation_ambiguousness(
    fk: sql::ForeignKeyWalker<'_>,
    ambiguous_relations: &mut HashSet<[sql::TableId; 2]>,
    input: InputContext,
) {
    let tables = table_ids_for_inline_relation(fk);

    if ambiguous_relations.contains(&tables) {
        return;
    };

    // The relation can be ambiguous because there are multiple relations between the one or two
    // models involved.
    let mut all_foreign_keys = fk
        .table()
        .foreign_keys()
        .chain(fk.referenced_table().foreign_keys())
        .filter(|other_fk| fks_are_distinct(fk, *other_fk));

    if all_foreign_keys.any(|other_fk| table_ids_for_inline_relation(other_fk) == tables) {
        ambiguous_relations.insert(tables);
        return;
    }

    // ...or because the relation field name conflicts with one of the scalar fields' name.
    let default_field_name = input.table_prisma_name(fk.referenced_table().id).prisma_name();
    if fk
        .constrained_columns()
        .any(|col| default_field_name == input.column_prisma_name(col.id).prisma_name())
    {
        ambiguous_relations.insert(tables);
    }
}

fn fks_are_distinct(a: sql::ForeignKeyWalker<'_>, b: sql::ForeignKeyWalker<'_>) -> bool {
    if a.id == b.id {
        return false;
    }

    let (a_cols, b_cols) = (a.constrained_columns(), b.constrained_columns());
    a_cols.len() != b_cols.len() || a_cols.zip(b_cols).any(|(col_a, col_b)| col_a.id != col_b.id)
}

fn table_ids_for_inline_relation(fk: sql::ForeignKeyWalker<'_>) -> [sql::TableId; 2] {
    sorted_table_ids(fk.table().id, fk.referenced_table().id)
}

fn table_ids_for_m2m_relation_table(table: sql::TableWalker<'_>) -> [sql::TableId; 2] {
    let mut referenced_tables = table.foreign_keys().map(|fk| fk.referenced_table().id);
    debug_assert!(
        referenced_tables.len() == 2,
        "Invariant: there are exactly two foreign keys because this is a Prisma many-to-many join table."
    );
    sorted_table_ids(referenced_tables.next().unwrap(), referenced_tables.next().unwrap())
}

fn sorted_table_ids(a: sql::TableId, b: sql::TableId) -> [sql::TableId; 2] {
    let mut tables = [a, b];
    tables.sort();
    tables
}

fn collect_duplicated_fks(table: sql::TableWalker<'_>, fks: &mut HashSet<sql::ForeignKeyId>) {
    let new_fks = table
        .foreign_keys()
        .enumerate()
        .filter(|(idx, left)| {
            let mut already_visited = table.foreign_keys().take(*idx);
            already_visited.any(|right| {
                let (left_constrained, right_constrained) = (left.constrained_columns(), right.constrained_columns());
                left_constrained.len() == right_constrained.len()
                    && left_constrained
                        .zip(right_constrained)
                        .all(|(left, right)| left.id == right.id)
                    && left
                        .referenced_columns()
                        .zip(right.referenced_columns())
                        .all(|(left, right)| left.id == right.id)
            })
        })
        .map(|(_, fk)| fk.id);
    fks.clear();
    fks.extend(new_fks)
}
