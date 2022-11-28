use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Diagnostic, Range, TextEdit, WorkspaceEdit,
};
use psl::parser_database::{
    ast::WithSpan,
    walkers::{CompleteInlineRelationWalker, ModelWalker, RelationFieldWalker, ScalarFieldWalker},
};
use std::collections::HashMap;

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
    params: &CodeActionParams,
    schema: &str,
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
    let text = create_missing_attribute(
        schema,
        relation.referencing_model(),
        relation.referencing_fields(),
        attribute_name,
    );

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    // The returned diagnostics are the ones we promise to fix with
    // the code action.
    let diagnostics = super::diagnostics_for_span(
        schema,
        &params.context.diagnostics,
        relation.referencing_field().ast_field().span(),
    );

    let action = CodeAction {
        title: String::from("Make referencing fields unique"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics,
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
    params: &CodeActionParams,
    schema: &str,
    relation: CompleteInlineRelationWalker<'_>,
) {
    if relation
        .referenced_model()
        .unique_criterias()
        .any(|crit| crit.contains_exactly_fields(relation.referenced_fields()))
    {
        return;
    }

    match (relation.referencing_fields().len(), relation.referenced_fields().len()) {
        (0, 0) => return,
        (a, b) if a != b => return,
        _ => (),
    }

    let attribute_name = "unique";
    let text = create_missing_attribute(
        schema,
        relation.referenced_model(),
        relation.referenced_fields(),
        attribute_name,
    );

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    // The returned diagnostics are the ones we promise to fix with
    // the code action.
    let diagnostics = super::diagnostics_for_span(
        schema,
        &params.context.diagnostics,
        relation.referencing_field().ast_field().span(),
    );

    let action = CodeAction {
        title: String::from("Make referenced field(s) unique"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics,
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
    params: &CodeActionParams,
    schema: &str,
    relation: RelationFieldWalker<'_>,
) {
    let Some(fields) = relation.fields() else { return; };
    if relation.model().indexes().any(|index| {
        index
            .fields()
            .zip(fields.clone())
            .all(|(index_field, relation_field)| index_field.field_id() == relation_field.field_id())
    }) {
        return;
    }

    let attribute_name = "index";
    let (new_text, range) = create_block_attribute(schema, relation.model(), fields, attribute_name);
    let text = TextEdit { range, new_text };

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let Some(span_diagnostics) = super::diagnostics_for_span(
        schema,
        &params.context.diagnostics,
        relation.relation_attribute().unwrap().span(),
    ) else { return; };

    let diagnostics = span_diagnostics
        .into_iter()
        .filter(|diag| diag.message.contains("relationMode = \"prisma\""))
        .collect::<Vec<Diagnostic>>();

    let action = CodeAction {
        title: String::from("Add an index for the relation's field(s)"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}

fn create_missing_attribute<'a>(
    schema: &str,
    model: ModelWalker<'a>,
    mut fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'a>> + 'a,
    attribute_name: &str,
) -> TextEdit {
    let (new_text, range) = if fields.len() == 1 {
        let new_text = format!(" @{attribute_name}");

        let field = fields.next().unwrap();
        let position = crate::position_after_span(field.ast_field().span(), schema);

        let range = Range {
            start: position,
            end: position,
        };

        (new_text, range)
    } else {
        let (new_text, range) = create_block_attribute(schema, model, fields, attribute_name);
        (new_text, range)
    };

    TextEdit { range, new_text }
}

fn create_block_attribute<'a>(
    schema: &str,
    model: ModelWalker<'a>,
    fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'a>> + 'a,
    attribute_name: &str,
) -> (String, Range) {
    let fields = fields.map(|f| f.name()).collect::<Vec<_>>().join(", ");

    let indentation = model.indentation();
    let newline = model.newline();
    let separator = if model.ast_model().attributes.is_empty() {
        newline.as_ref()
    } else {
        ""
    };
    let new_text = format!("{separator}{indentation}@@{attribute_name}([{fields}]){newline}}}");

    let start = crate::offset_to_position(model.ast_model().span().end - 1, schema);
    let end = crate::offset_to_position(model.ast_model().span().end, schema);

    let range = Range { start, end };

    (new_text, range)
}
