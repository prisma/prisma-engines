mod table;

pub(crate) use table::TableFormat;

use crate::ast::{self, WithDocumentation};

/// Get the sort order for an attribute, in the canonical sorting order.
pub(crate) fn get_sort_index_of_attribute(attribute_name: &str) -> usize {
    let correct_order: &[&str] = &[
        "id",
        "unique",
        "default",
        "updatedAt",
        "index",
        "fulltext",
        "map",
        "relation",
        "ignore",
    ];

    correct_order
        .iter()
        .position(|p| attribute_name.trim_start_matches('@').starts_with(p))
        .unwrap_or(usize::MAX)
}

pub trait LineWriteable {
    fn write(&mut self, param: &str);
    fn end_line(&mut self);
}

pub struct Renderer {
    pub stream: String,
    indent: usize,
    indent_width: usize,
}

impl Renderer {
    pub fn new(indent_width: usize) -> Renderer {
        Renderer {
            stream: String::new(),
            indent: 0,
            indent_width,
        }
    }

    pub fn render(&mut self, datamodel: &ast::SchemaAst) {
        for top in datamodel.tops.iter() {
            match top {
                ast::Top::CompositeType(ct) => self.render_composite_type(ct),
                ast::Top::Model(model) => self.render_model(model),
                ast::Top::Enum(enm) => self.render_enum(enm),
                ast::Top::Source(_) | ast::Top::Generator(_) => unreachable!(),
            }
        }
    }

    fn render_documentation(
        target: &mut dyn LineWriteable,
        documentation: Option<&ast::Comment>,
        is_commented_out: bool,
    ) {
        if let Some(doc) = documentation {
            for line in doc.text.split('\n') {
                // We comment out objects in introspection. Those are put into `//` comments.
                // We use the documentation on the object to render an explanation for why that happened. It's nice if this explanation is also in a `//` instead of a `///` comment.
                if is_commented_out {
                    target.write("// ");
                } else {
                    target.write("/// ");
                }
                target.write(line);
                target.end_line();
            }
        }
    }

    fn render_model(&mut self, model: &ast::Model) {
        let comment_out = if model.commented_out { "// " } else { "" };
        Self::render_documentation(self, model.documentation.as_ref(), model.is_commented_out());
        self.write(comment_out);
        self.write("model ");
        self.write(&model.name.name);
        self.write(" {");
        self.end_line();

        for field in &model.fields {
            Self::render_field(self, field, model.commented_out);
        }

        if !model.attributes.is_empty() {
            self.end_line();
            // sort attributes
            let attributes = &model.attributes;
            for attribute in attributes {
                self.render_block_attribute(attribute, comment_out);
            }
        }

        self.write(format!("{}{}", comment_out, "}").as_ref());
        self.end_line();
    }

    fn render_composite_type(&mut self, type_def: &ast::CompositeType) {
        Self::render_documentation(self, type_def.documentation.as_ref(), type_def.is_commented_out());

        self.write("type ");
        self.write(&type_def.name.name);
        self.write(" {\n");

        for field in &type_def.fields {
            Self::render_field(self, field, false);
        }

        self.write("}\n");
    }

    fn render_enum(&mut self, enm: &ast::Enum) {
        Self::render_documentation(self, enm.documentation.as_ref(), enm.is_commented_out());

        self.write("enum ");
        self.write(&enm.name.name);
        self.write(" {\n");

        for value in &enm.values {
            let commented_out = if value.commented_out { "// " } else { "" };
            self.write(commented_out);
            self.write(&value.name.name);
            if !value.attributes.is_empty() {
                for attribute in &value.attributes {
                    self.write(" ");
                    Self::render_field_attribute(self, attribute);
                }
            }

            if let Some(comment) = &value.documentation {
                self.write(&format!(" // {}", comment.text.as_str()));
            }

            self.end_line();
        }

        if !enm.attributes.is_empty() {
            self.end_line();
            let attributes = &enm.attributes;
            for attribute in attributes {
                self.render_block_attribute(attribute, "");
            }
        }

        self.write("}\n");
    }

    fn render_field(target: &mut dyn LineWriteable, field: &ast::Field, is_commented_out: bool) {
        Self::render_documentation(target, field.documentation.as_ref(), field.is_commented_out);

        let commented_out = if field.is_commented_out || is_commented_out {
            "// "
        } else {
            ""
        };

        target.write(commented_out);
        target.write(&field.name.name);
        target.write(" ");

        // Type
        Self::render_field_type(target, &field.field_type);
        Self::render_field_arity(target, &field.arity);

        // Attributes
        let attributes = &field.attributes;
        for attribute in attributes {
            target.write(" ");
            Self::render_field_attribute(target, attribute);
        }

        target.end_line();
    }

    fn render_field_arity(target: &mut dyn LineWriteable, field_arity: &ast::FieldArity) {
        match field_arity {
            ast::FieldArity::List => target.write("[]"),
            ast::FieldArity::Optional => target.write("?"),
            ast::FieldArity::Required => {}
        };
    }

    fn render_field_attribute(target: &mut dyn LineWriteable, attribute: &ast::Attribute) {
        target.write("@");
        target.write(&attribute.name.name);

        if !attribute.arguments.is_empty() {
            target.write("(");
            Self::render_arguments(target, &attribute.arguments);
            target.write(")");
        }
    }

    fn render_field_type(target: &mut dyn LineWriteable, field_type: &ast::FieldType) {
        match field_type {
            ast::FieldType::Supported(ft) => {
                target.write(&ft.name);
            }
            ast::FieldType::Unsupported(lit, _) => {
                target.write("Unsupported(\"");
                target.write(lit);
                target.write("\")");
            }
        }
    }

    fn render_block_attribute(&mut self, attribute: &ast::Attribute, commented_out: &str) {
        self.write(format!("{}@@", commented_out).as_ref());
        self.write(&attribute.name.name);

        if !attribute.arguments.is_empty() {
            self.write("(");
            Self::render_arguments(self, &attribute.arguments);
            self.write(")");
        }

        self.end_line();
    }

    fn render_arguments(target: &mut dyn LineWriteable, args: &ast::ArgumentsList) {
        for (idx, arg) in args.iter().enumerate() {
            if idx > 0 {
                target.write(", ");
            }
            Self::render_argument(target, arg);
        }
    }

    fn render_argument(target: &mut dyn LineWriteable, arg: &ast::Argument) {
        if let Some(arg_name) = &arg.name {
            target.write(&arg_name.name);
            target.write(": ");
        }
        Self::render_value(target, &arg.value);
    }

    pub fn render_value_to_string(val: &ast::Expression) -> String {
        let mut builder = String::new();
        Self::render_value(&mut builder, val);
        builder
    }

    fn render_value(target: &mut dyn LineWriteable, val: &ast::Expression) {
        match val {
            ast::Expression::Array(vals, _) => Self::render_expression_array(target, vals),
            ast::Expression::ConstantValue(val, _) => target.write(val),
            ast::Expression::NumericValue(val, _) => target.write(val),
            ast::Expression::StringValue(val, _) => Self::render_str(target, val),
            ast::Expression::Function(name, args, _) => Self::render_func(target, name, args),
        };
    }

    fn render_func(target: &mut dyn LineWriteable, name: &str, args: &ast::ArgumentsList) {
        target.write(name);
        target.write("(");
        Self::render_arguments(target, args);
        target.write(")");
    }

    pub(crate) fn indent_up(&mut self) {
        self.indent += 1
    }

    pub(crate) fn indent_down(&mut self) {
        if self.indent == 0 {
            panic!("Indentation error.")
        }
        self.indent -= 1
    }

    fn render_expression_array(target: &mut dyn LineWriteable, vals: &[ast::Expression]) {
        target.write("[");
        for (idx, arg) in vals.iter().enumerate() {
            if idx > 0 {
                target.write(", ");
            }
            Self::render_value(target, arg);
        }
        target.write("]");
    }

    /// https://datatracker.ietf.org/doc/html/rfc8259#section-7
    pub fn render_str(target: &mut dyn LineWriteable, param: &str) {
        target.write("\"");
        for c in param.char_indices() {
            match c {
                (_, '\t') => target.write("\\t"),
                (_, '\n') => target.write("\\n"),
                (_, '"') => target.write("\\\""),
                (_, '\r') => target.write("\\r"),
                (_, '\\') => target.write("\\\\"),
                // Control characters
                (_, c) if c.is_ascii_control() => {
                    let mut b = [0];
                    c.encode_utf8(&mut b);
                    let formatted = format!("\\u{:04x}", b[0]);
                    target.write(&formatted)
                }
                (start, other) => target.write(&param[start..(start + other.len_utf8())]),
            }
        }
        target.write("\"");
    }
}

impl LineWriteable for Renderer {
    fn write(&mut self, param: &str) {
        if self.stream.is_empty() || self.stream.ends_with('\n') {
            for _ in 0..(self.indent * self.indent_width) {
                self.stream.push(' ');
            }
        }

        self.stream.push_str(param);
    }

    fn end_line(&mut self) {
        self.stream.push('\n');
    }
}

impl LineWriteable for String {
    fn write(&mut self, param: &str) {
        self.push_str(param);
    }

    fn end_line(&mut self) {
        panic!("cannot end line in string builder");
    }
}
