use crate::ast::{self, top_idx_to_top_id, traits::*};

impl ast::SchemaAst {
    /// Find the AST node at the given position (byte offset).
    pub fn find_at_position(&self, position: usize) -> SchemaPosition<'_> {
        self.find_top_at_position(position)
            .map(|top_id| match top_id {
                ast::TopId::Model(model_id) => {
                    SchemaPosition::Model(model_id, ModelPosition::new(&self[model_id], position))
                }
                // Falling back to TopLevel as "not implemented"
                _ => SchemaPosition::TopLevel,
            })
            // If no top matched, we're in between top-level items. This is normal and expected.
            .unwrap_or(SchemaPosition::TopLevel)
    }

    /// Do a binary search for the `Top` at the given byte offset.
    pub fn find_top_at_position(&self, position: usize) -> Option<ast::TopId> {
        use std::cmp::Ordering;

        let top_idx = self.tops.binary_search_by(|top| {
            let span = top.span();

            if span.start > position {
                Ordering::Greater
            } else if span.end < position {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });

        top_idx.map(|idx| top_idx_to_top_id(idx, &self.tops[idx])).ok()
    }
}

/// A cursor position in a schema.
#[derive(Debug)]
pub enum SchemaPosition<'ast> {
    /// In-between top-level items
    TopLevel,
    /// In a model
    Model(ast::ModelId, ModelPosition<'ast>),
}

/// A cursor position in a context.
#[derive(Debug)]
pub enum ModelPosition<'ast> {
    /// In the model, but not somewhere more specific.
    Model,
    /// In an attribute (attr name, attr index, position).
    ModelAttribute(&'ast str, usize, AttributePosition<'ast>),
    /// In a field.
    Field(ast::FieldId, FieldPosition<'ast>),
}

impl<'ast> ModelPosition<'ast> {
    fn new(model: &'ast ast::Model, position: usize) -> Self {
        for (field_id, field) in model.iter_fields() {
            if field.span().contains(position) {
                return ModelPosition::Field(field_id, FieldPosition::new(field, position));
            }
        }

        for (attr_id, attr) in model.attributes.iter().enumerate() {
            if attr.span().contains(position) {
                return ModelPosition::ModelAttribute(&attr.name.name, attr_id, AttributePosition::new(attr, position));
            }
        }

        ModelPosition::Model
    }
}

/// In a scalar field.
#[derive(Debug)]
pub enum FieldPosition<'ast> {
    /// Nowhere specific inside the field
    Field,
    /// In an attribute. (name, idx, optional arg)
    Attribute(&'ast str, usize, Option<&'ast str>),
}

impl<'ast> FieldPosition<'ast> {
    fn new(field: &'ast ast::Field, position: usize) -> FieldPosition<'ast> {
        for (attr_idx, attr) in field.attributes.iter().enumerate() {
            if attr.span().contains(position) {
                // We can't go by Span::contains() because we also care about the empty space
                // between arguments and that's hard to capture in the pest grammar.
                let mut spans: Vec<(Option<&str>, ast::Span)> = attr
                    .arguments
                    .iter()
                    .map(|arg| (arg.name.as_ref().map(|n| n.name.as_str()), arg.span()))
                    .chain(
                        attr.arguments
                            .empty_arguments
                            .iter()
                            .map(|arg| (Some(arg.name.name.as_str()), arg.name.span())),
                    )
                    .collect();
                spans.sort_by_key(|(_, span)| span.start);
                let mut arg_name = None;

                for (name, _) in spans.iter().take_while(|(_, span)| span.start < position) {
                    arg_name = Some(*name);
                }

                // If the cursor is after a trailing comma, we're not in an argument.
                if let Some(span) = attr.arguments.trailing_comma {
                    if position > span.start {
                        arg_name = None;
                    }
                }

                return FieldPosition::Attribute(attr.name(), attr_idx, arg_name.flatten());
            }
        }

        FieldPosition::Field
    }
}

/// In an model attribute definition
#[derive(Debug)]
pub enum AttributePosition<'ast> {
    /// Nowhere specific inside the attribute (attribute name)
    Attribute,
    /// In an argument. (argument name)
    Argument(&'ast str),
    /// In an function argument. (function name, argument name)
    FunctionArgument(&'ast str, &'ast str),
}

impl<'ast> AttributePosition<'ast> {
    fn new(attr: &'ast ast::Attribute, position: usize) -> Self {
        if attr.span().contains(position) {
            // We can't go by Span::contains() because we also care about the empty space
            // between arguments and that's hard to capture in the pest grammar.
            let mut spans: Vec<(Option<&str>, ast::Span)> = attr
                .arguments
                .iter()
                .map(|arg| (arg.name.as_ref().map(|n| n.name.as_str()), arg.span()))
                .chain(
                    attr.arguments
                        .empty_arguments
                        .iter()
                        .map(|arg| (Some(arg.name.name.as_str()), arg.name.span())),
                )
                .collect();

            spans.sort_by_key(|(_, span)| span.start);

            let mut arg_name = None;
            for (name, _) in spans.iter().take_while(|(_, span)| span.start < position) {
                arg_name = Some(*name);
            }

            // If the cursor is after a trailing comma, we're not in an argument.
            if let Some(span) = attr.arguments.trailing_comma {
                if position > span.start {
                    arg_name = None;
                }
            }

            if let Some(arg_name) = arg_name.flatten() {
                return Self::Argument(arg_name);
            }

            if let Some(arg) = attr.arguments.iter().find(|arg| arg.span().contains(position)) {
                if let ExpressionPosition::FunctionArgument(fun, name) = ExpressionPosition::new(&arg.value, position) {
                    return Self::FunctionArgument(fun, name);
                }
            }
        }

        Self::Attribute
    }
}

#[derive(Debug)]
pub enum ExpressionPosition<'ast> {
    Expression,
    Value(&'ast str),
    Function(&'ast str),
    FunctionArgument(&'ast str, &'ast str),
}

impl<'ast> ExpressionPosition<'ast> {
    fn new(expr: &'ast ast::Expression, position: usize) -> Self {
        match expr {
            ast::Expression::NumericValue(val, span) if span.contains(position) => Self::Value(val),
            ast::Expression::StringValue(val, span) if span.contains(position) => Self::Value(val),
            ast::Expression::ConstantValue(val, span) if span.contains(position) => Self::Value(val),
            ast::Expression::Function(name, args, span) if span.contains(position) => {
                let mut spans: Vec<(Option<&str>, ast::Span)> = args
                    .arguments
                    .iter()
                    .map(|arg| (arg.name.as_ref().map(|n| n.name.as_str()), arg.span()))
                    .chain(
                        args.empty_arguments
                            .iter()
                            .map(|arg| (Some(arg.name.name.as_str()), arg.name.span())),
                    )
                    .collect();

                spans.sort_by_key(|(_, span)| span.start);

                let mut arg_name = None;
                for (name, _) in spans.iter().take_while(|(_, span)| span.start < position) {
                    arg_name = Some(*name);
                }

                // If the cursor is after a trailing comma, we're not in an argument.
                if let Some(span) = args.trailing_comma {
                    if position > span.start {
                        arg_name = None;
                    }
                }

                if let Some(arg_name) = arg_name.flatten() {
                    Self::FunctionArgument(name, arg_name)
                } else {
                    Self::Function(name)
                }
            }
            ast::Expression::Array(exprs, span) if span.contains(position) => {
                for expr in exprs.iter() {
                    match ExpressionPosition::new(expr, position) {
                        ExpressionPosition::Expression => (),
                        e => return e,
                    }
                }

                Self::Expression
            }
            _ => Self::Expression,
        }
    }
}
