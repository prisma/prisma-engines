use std::collections::HashMap;

use datamodel::parser_database::walkers::CompleteInlineRelationWalker;
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Diagnostic, DiagnosticSeverity, Range, TextEdit,
    WorkspaceEdit,
};

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

    let mut fields = relation.referencing_fields();

    let (new_text, start, end, diagnostic) = if fields.len() == 1 {
        let new_text = String::from(" @unique");

        let diagnostic = {
            Diagnostic {
                message: String::from("The field has to be unique due to a relation"),
                range: crate::span_to_range(relation.referencing_field().ast_field().span, schema),
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            }
        };

        let field = fields.next().unwrap();
        let range = crate::span_to_range(field.ast_field().span, schema);

        (new_text, range.end, range.end, diagnostic)
    } else {
        let fields = fields.map(|f| f.name()).collect::<Vec<_>>().join(", ");
        let model = relation.referencing_model();

        let new_text = indoc::formatdoc! {r#"
          @@unique([{fields}])
        }}"#};

        let diagnostic = {
            Diagnostic {
                message: format!("The model needs a unique constraint (fields: {fields}) due to a relation"),
                range: crate::span_to_range(relation.referencing_field().ast_field().span, schema),
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            }
        };

        let start = crate::offset_to_position(model.ast_model().span.end - 1, schema).unwrap();
        let end = crate::offset_to_position(model.ast_model().span.end, schema).unwrap();

        (new_text, start, end, diagnostic)
    };

    let text = TextEdit {
        range: Range { start, end },
        new_text,
    };

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let action = CodeAction {
        title: String::from("Make referencing fields unique"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(vec![diagnostic]),
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
        .any(|crit| crit.contains_exactly_fields(relation.referencing_fields()))
    {
        return;
    }

    match (relation.referencing_fields().len(), relation.referenced_fields().len()) {
        (0, 0) => return,
        (a, b) if a != b => return,
        _ => (),
    }

    let mut fields = relation.referenced_fields();

    let (new_text, start, end, diagnostic) = if fields.len() == 1 {
        let new_text = String::from(" @unique");

        let diagnostic = {
            Diagnostic {
                message: String::from("The field has to be unique due to a relation"),
                range: crate::span_to_range(relation.referencing_field().ast_field().span, schema),
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            }
        };

        let field = fields.next().unwrap();
        let start = crate::offset_to_position(field.ast_field().span.end - 1, schema).unwrap();

        (new_text, start, start, diagnostic)
    } else {
        let model = relation.referenced_model();
        let fields = fields.map(|f| f.name()).collect::<Vec<_>>().join(", ");

        let new_text = indoc::formatdoc! {r#"
          @@unique([{fields}])
        }}"#};

        let diagnostic = {
            Diagnostic {
                message: format!("The model needs a unique constraint (for fields: {fields}) due to a relation"),
                range: crate::span_to_range(relation.referencing_field().ast_field().span, schema),
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            }
        };

        let start = crate::offset_to_position(model.ast_model().span.end - 1, schema).unwrap();
        let end = crate::offset_to_position(model.ast_model().span.end, schema).unwrap();

        (new_text, start, end, diagnostic)
    };

    let text = TextEdit {
        range: Range { start, end },
        new_text,
    };

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let action = CodeAction {
        title: String::from("Make referenced field(s) unique"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(vec![diagnostic]),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action));
}
