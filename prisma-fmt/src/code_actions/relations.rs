use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, TextEdit, WorkspaceEdit};
use psl::parser_database::{
    ast::WithSpan,
    walkers::{CompleteInlineRelationWalker, InlineRelationWalker, RelationFieldWalker},
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
    changes.insert(context.params.text_document.uri.clone(), vec![text]);

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

/// If the referencing side of the relation does not include
/// a complete relation attribute.
///
/// If it includes no relation attribute:
///
/// ```prisma
/// model interm {
///     id Int @id
///     forumId Int
///     forum   Forum
/// //                ^^^  suggests `@relation(fields: [], references: [])`
/// }
/// ```
///
/// If it includes an empty relation attribute:
///
/// ```prisma
/// model interm {
///     id Int @id
///     forumId Int
///     forum   Forum @relation(  )
/// //                         ^^^  suggests `fields: [], references: []``
/// }
/// ```
///
/// ```prisma
///
/// model Forum {
///     id   Int    @id
///     name String
///
///     interm interm[]
/// }
/// ```
pub(super) fn add_referencing_side_relation(
    actions: &mut Vec<CodeActionOrCommand>,
    ctx: &CodeActionsContext<'_>,
    relation: InlineRelationWalker<'_>,
) {
    let Some(initiating_field) = relation.forward_relation_field() else {
        return;
    };

    // * Full example diagnostic message:
    // ! Error parsing attribute "@relation":
    // ! The relation field `forum` on Model `Interm` must specify
    // ! the `fields` argument in the @relation attribute.
    // ! You can run `prisma format` to fix this automatically.
    let mut diagnostics = ctx.diagnostics_for_span_with_message(
        initiating_field.ast_field().span(),
        "must specify the `fields` argument in the @relation attribute.",
    );

    // ? (@druue) We seem to have a slightly different message for effectively the same schema state
    // * Full example diagnostic message:
    // ! Error parsing attribute "@relation":
    // ! The relation fields `wife` on Model `User` and `husband` on Model `User`
    // ! do not provide the `fields` argument in the @relation attribute.
    // ! You have to provide it on one of the two fields.
    diagnostics.extend(ctx.diagnostics_for_span_with_message(
        initiating_field.ast_field().span(),
        "do not provide the `fields` argument in the @relation attribute.",
    ));

    if diagnostics.is_empty() {
        return;
    }

    let pk = relation.referenced_model().primary_key();
    let newline = relation.referenced_model().newline();

    let Some((reference_ids, field_ids, fields)) = pk.map(|pk| {
        let (names, (field_ids, fields)): (Vec<&str>, (Vec<String>, Vec<String>)) = pk
            .fields()
            .map(|f| {
                let field_name = f.name();
                let field_id = format!("{}{}", initiating_field.ast_field().name(), field_name);
                let field_full = format!("{} {}?", field_id, f.ast_field().field_type.name());

                (field_name, (field_id, field_full))
            })
            .unzip();

        (
            names.join(", "),
            field_ids.join(", "),
            format!("\n{}{}", fields.join(newline.as_ref()), newline),
        )
    }) else {
        return;
    };

    let references = format!("references: [{reference_ids}]");
    let fields_arg = format!("fields: [{field_ids}]");

    // * In the prisma-fmt incarnation of this, we assume:
    // * - fields contains a field with the name `referenced_modelId`
    // * - references contains a field named `id`
    let (range, new_text) = match initiating_field.relation_attribute() {
        Some(attr) => {
            let name = attr
                .arguments
                .arguments
                .iter()
                .find(|arg| arg.value.is_string())
                .map_or(Default::default(), |arg| format!("{arg}, "));

            let new_text = format!("@relation({}{}, {})", name, fields_arg, references);
            let range = super::span_to_range(attr.span(), ctx.initiating_file_source());

            (range, new_text)
        }
        None => {
            let new_text = format!(
                " @relation({}, {}){}",
                fields_arg,
                references,
                initiating_field.model().newline()
            );
            let range = super::range_after_span(initiating_field.ast_field().span(), ctx.initiating_file_source());

            (range, new_text)
        }
    };

    let mut changes: HashMap<lsp_types::Url, Vec<TextEdit>> = HashMap::new();
    changes.insert(
        ctx.params.text_document.uri.clone(),
        vec![
            TextEdit { range, new_text },
            TextEdit {
                range: super::range_after_span(initiating_field.ast_field().span(), ctx.initiating_file_source()),
                new_text: fields,
            },
        ],
    );

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let action = CodeAction {
        title: String::from("Add relation attribute for relation field"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}

pub(super) fn make_referencing_side_many(
    actions: &mut Vec<CodeActionOrCommand>,
    ctx: &CodeActionsContext<'_>,
    relation: CompleteInlineRelationWalker<'_>,
) {
    let initiating_field = relation.referencing_field();

    // * Full example diagnostic message:
    // ! Error parsing attribute "@relation":
    // ! The relation field `forum` on Model `Interm` must specify
    // ! the `fields` argument in the @relation attribute.
    // ! You can run `prisma format` to fix this automatically.
    let diagnostics = ctx.diagnostics_for_span_with_message(
        initiating_field.ast_field().span(),
        "must specify the `fields` argument in the @relation attribute.",
    );

    if diagnostics.is_empty() {
        return;
    }

    let text = match initiating_field.relation_attribute() {
        Some(_) => return,
        None => {
            let new_text = format!("[]{}", initiating_field.model().newline());
            let range = super::range_after_span(initiating_field.ast_field().span(), ctx.initiating_file_source());

            TextEdit { range, new_text }
        }
    };

    let mut changes: HashMap<lsp_types::Url, Vec<TextEdit>> = HashMap::new();
    changes.insert(ctx.params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let action = CodeAction {
        title: String::from("Mark relation field as many `[]`"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
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

    let range = super::range_after_span(relation.model().ast_model().span(), context.initiating_file_source());
    let text = TextEdit {
        range,
        new_text: formatted_attribute,
    };

    let mut changes = HashMap::new();
    changes.insert(context.params.text_document.uri.clone(), vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let diagnostics = context.diagnostics_for_span_with_message(
        relation.relation_attribute().unwrap().span(),
        "relationMode = \"prisma\"",
    );

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
