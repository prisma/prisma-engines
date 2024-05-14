use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, TextEdit, WorkspaceEdit};
use psl::parser_database::{
    ast::WithSpan,
    walkers::{CompleteInlineRelationWalker, RelationFieldWalker},
};
use std::collections::HashMap;

use super::{format_block_attribute, parse_url, CodeActionsContext};

/// If the referencing side of the one-to-one relation does not point
/// to a unique constraint, the action adds the attribute.
///
/// If referencing a single field:
///
/// ```ignore
/// model A {
///   id    Int @id
///   field Int @unique
///   b     B   @relation("foo")
/// }
///
/// model B {
///   id  Int  @id
///   aId Int? // <- suggest @unique here
///   a   A    @relation("foo", fields: [aId], references: [field])
/// }
/// ```
///
/// If the referencing multiple fields:
///
/// ```ignore
/// model A {
///   id     Int @id
///   field1 Int
///   field2 Int
///   b      B   @relation("foo")
///
///   // <- suggest @@unique([field1, field2]) here
/// }
///
/// model B {
///   id   Int  @id
///   aId1 Int?
///   aId2 Int?
///   a    A    @relation("foo", fields: [aId1, aId2], references: [field1, field2])
///
///   @@unique([aId1, aId2])
/// }
/// ```
pub(super) fn add_referencing_side_unique(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    relation: CompleteInlineRelationWalker<'_>,
) {
    if relation
        .referencing_model()
        .unique_criterias()
        .any(|crit| crit.contains_exactly_fields(relation.referencing_fields()))
    {
        return;
    }

    match (relation.referencing_fields().len(), relation.referenced_fields().len()) {
        (0, 0) => return,
        (a, b) if a != b => return,
        _ => (),
    }

    let attribute_name = "unique";
    let text = super::create_missing_attribute(
        context.initiating_file_source(),
        relation.referencing_model(),
        relation.referencing_fields(),
        attribute_name,
    );

    let mut changes = HashMap::new();
    changes.insert(context.lsp_params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    // The returned diagnostics are the ones we promise to fix with
    // the code action.
    let diagnostics = context
        .diagnostics_for_span(relation.referencing_field().ast_field().span())
        .cloned()
        .collect();

    let action = CodeAction {
        title: String::from("Make referencing fields unique"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action));
}

/// If the referenced side of the relation does not point to a unique
/// constraint, the action adds the attribute.
///
/// If referencing a single field:
///
/// ```ignore
/// model A {
///   id    Int @id
///   field Int // <- suggests @unique here
///   bs    B[]
/// }
///
/// model B {
///   id  Int @id
///   aId Int
///   a   A   @relation(fields: [aId], references: [field])
/// }
/// ```
///
/// If the referencing multiple fields:
///
/// ```ignore
/// model A {
///   id     Int @id
///   field1 Int
///   field2 Int
///   bs     B[]
///   // <- suggest @@unique([field1, field2]) here
/// }
///
/// model B {
///   id   Int @id
///   aId1 Int
///   aId2 Int
///   a    A   @relation(fields: [aId1, aId2], references: [field1, field2])
/// }
/// ```
pub(super) fn add_referenced_side_unique(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    relation: CompleteInlineRelationWalker<'_>,
) {
    if relation
        .referenced_model()
        .unique_criterias()
        .any(|crit| crit.contains_exactly_fields(relation.referenced_fields()))
    {
        return;
    }

    let file_id = relation.referenced_model().ast_model().span().file_id;
    let file_uri = relation.db.file_name(file_id);
    let file_content = relation.db.source(file_id);

    match (relation.referencing_fields().len(), relation.referenced_fields().len()) {
        (0, 0) => return,
        (a, b) if a != b => return,
        _ => (),
    }

    let attribute_name = "unique";
    let text = super::create_missing_attribute(
        file_content,
        relation.referenced_model(),
        relation.referenced_fields(),
        attribute_name,
    );

    let mut changes = HashMap::new();
    let Ok(url) = parse_url(file_uri) else {
        return;
    };
    changes.insert(url, vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    // The returned diagnostics are the ones we promise to fix with
    // the code action.
    let diagnostics = context
        .diagnostics_for_span(relation.referencing_field().ast_field().span())
        .cloned()
        .collect();

    let action = CodeAction {
        title: String::from("Make referenced field(s) unique"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action));
}

/// For schema's with emulated relations,
/// If the referenced side of the relation does not point to a unique
/// constraint, the action adds the attribute.
///
/// If referencing a single field:
///
/// ```ignore
/// model A {
///     id      Int @id
///     field1  B   @relation(fields: [bId], references: [id])
///                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ // Warn
///     bId     Int
///
///     // <- suggest @@index([bId]) here
/// }
///
/// model B {
///     id Int @id
///     as A[]
/// }
/// ```
///
/// If referencing multiple fields:
///
/// ```ignore
/// model A {
///     id      Int @id
///     field1  B   @relation(fields: [bId1, bId2, bId3], references: [id1, id2, id3])
///                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ // Warn
///     bId1    Int
///     bId2    Int
///     bId3    Int
///
///     // <- suggest @@index([bId1, bId2, bId3]) here
/// }
///
/// model B {
///     id1 Int
///     id2 Int
///     id3 Int
///     as  A[]
///
///     @@id([id1, id2, id3])
/// }
/// ```
pub(super) fn add_index_for_relation_fields(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    relation: RelationFieldWalker<'_>,
) {
    let fields = match relation.fields() {
        Some(fields) => fields,
        None => return,
    };
    if relation.model().indexes().any(|index| {
        index
            .fields()
            .zip(fields.clone())
            .all(|(index_field, relation_field)| index_field.field_id() == relation_field.field_id())
    }) {
        return;
    }

    let fields = fields.map(|f| f.name()).collect::<Vec<_>>().join(", ");

    let attribute_name = "index";
    let attribute = format!("{attribute_name}([{fields}])");
    let formatted_attribute = format_block_attribute(
        &attribute,
        relation.model().indentation(),
        relation.model().newline(),
        &relation.model().ast_model().attributes,
    );

    let range = super::range_after_span(context.initiating_file_source(), relation.model().ast_model().span());
    let text = TextEdit {
        range,
        new_text: formatted_attribute,
    };

    let mut changes = HashMap::new();
    changes.insert(context.lsp_params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let diagnostics = context
        .diagnostics_for_span_with_message(relation.relation_attribute().unwrap().span, "relationMode = \"prisma\"");

    if diagnostics.is_empty() {
        return;
    }

    let action = CodeAction {
        title: String::from("Add an index for the relation's field(s)"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
