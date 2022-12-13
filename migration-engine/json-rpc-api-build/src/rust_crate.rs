use crate::{Api, CrateResult};
use heck::*;
use std::{borrow::Cow, fs::File, io::Write as _, path::Path};

pub(crate) fn generate_rust_crate(out_dir: &Path, api: &Api) -> CrateResult {
    let librs = out_dir.join("methods.rs");
    let mut librs = std::io::BufWriter::new(File::create(&librs)?);
    let mut method_names: Vec<&str> = api.methods.keys().map(String::as_str).collect();
    method_names.sort_unstable();

    librs.write_all(b"pub mod json_rpc {\n")?;
    librs.write_all(b"//! The JSON-RPC API definition.\n//!\n//! ## Methods\n//!\n")?;

    for method_name in &method_names {
        let method = &api.methods[*method_name];

        writeln!(librs, "//!\n//! ### 🔌 {method_name}\n")?;
        writeln!(
            librs,
            "//! ➡️  [{request_name}](./types/struct.{request_name}.html)\n//!",
            request_name = method.request_shape.to_camel_case()
        )?;
        writeln!(
            librs,
            "//! ↩️  [{response_name}](./types/struct.{response_name}.html)\n//!",
            response_name = method.response_shape.to_camel_case()
        )?;

        if let Some(description) = &method.description {
            for line in description.lines() {
                writeln!(librs, "//! {}", line)?;
            }
        }
    }

    librs.write_all(
        b"/// String constants for method names.\npub mod method_names {\n/// Exhaustive list of the names of all JSON-RPC methods.\npub const METHOD_NAMES: &[&str] = &[",
    )?;

    for method_name in &method_names {
        writeln!(librs, "    \"{}\",", method_name)?;
    }

    writeln!(librs, "];")?;

    for method_name in &method_names {
        writeln!(
            librs,
            "/// {method_name}\npub const {}: &str = \"{method_name}\";",
            method_name.to_snake_case().to_shouty_snake_case()
        )?;
    }

    librs.write_all(b"}\n")?; // close method_names

    generate_types_rs(&mut librs, api)?;

    librs.write_all(b"}\n")?;

    Ok(())
}

fn generate_types_rs(mut file: impl std::io::Write, api: &Api) -> CrateResult {
    file.write_all(
        b"/// API type definitions used by the methods.\n#[allow(missing_docs)] pub mod types {\nuse serde::{Serialize, Deserialize};\n\n",
    )?;

    for (type_name, record_type) in &api.record_shapes {
        if let Some(description) = &record_type.description {
            for line in description.lines() {
                writeln!(file, "/// {}", line)?;
            }
        }

        if let Some(example) = &record_type.example {
            file.write_all(b"/// ### Example\n///\n/// ```ignore")?;
            for line in example.lines() {
                file.write_all(b"\n/// ")?;
                file.write_all(line.as_bytes())?;
            }
            file.write_all(b"\n/// ```\n")?;
        }

        writeln!(
            file,
            "#[derive(Serialize, Deserialize, Debug)]\npub struct {} {{",
            rustify_type_name(type_name)
        )?;
        for (field_name, field) in &record_type.fields {
            if let Some(description) = &field.description {
                for line in description.lines() {
                    writeln!(file, "    /// {}", line)?;
                }
            }
            let type_name = rustify_type_name(&field.shape);
            let type_name: Cow<'static, str> = match (field.is_list, field.is_nullable) {
                (true, true) => format!("Option<Vec<{}>>", type_name).into(),
                (false, true) => format!("Option<{}>", type_name).into(),
                (true, false) => format!("Vec<{}>", type_name).into(),
                (false, false) => type_name,
            };
            let field_name_sc = field_name.to_snake_case();
            if &field_name_sc != field_name {
                writeln!(file, "    ///\n    /// JSON name: {}", field_name)?;
                writeln!(file, "    #[serde(rename = \"{}\")]", field_name)?;
            }

            writeln!(file, "    pub {}: {},", field_name_sc, type_name)?;
        }
        writeln!(file, "}}\n")?;
    }

    for (type_name, variants) in &api.enum_shapes {
        if let Some(description) = &variants.description {
            for line in description.lines() {
                writeln!(file, "/// {}", line)?;
            }
        }

        writeln!(
            file,
            "#[derive(Serialize, Deserialize, Debug)]\n#[serde(tag = \"tag\")]\npub enum {} {{",
            rustify_type_name(type_name)
        )?;

        for (variant_name, variant) in &variants.variants {
            if let Some(description) = &variant.description {
                for line in description.lines() {
                    writeln!(file, "/// {}", line)?;
                }
            }

            let cc_variant_name = variant_name.to_camel_case();

            if cc_variant_name.as_str() != variant_name {
                writeln!(file, "///\n/// JSON name: {}", variant_name)?;
                writeln!(file, "#[serde(rename = \"{}\")]", variant_name)?;
            }

            if let Some(shape) = &variant.shape {
                writeln!(file, "    {}({}),", cc_variant_name, rustify_type_name(shape))?;
            } else {
                writeln!(file, "    {},", cc_variant_name)?;
            }
        }

        file.write_all(b"}\n")?;
    }

    file.write_all(b"}\n")?;

    Ok(())
}

fn rustify_type_name(name: &str) -> Cow<'static, str> {
    match name {
        "bool" => Cow::Borrowed("bool"),
        "u32" => Cow::Borrowed("u32"),
        "isize" => Cow::Borrowed("isize"),
        "string" => Cow::Borrowed("String"),
        "serde_json::Value" => Cow::Borrowed("serde_json::Value"),
        other => other.to_camel_case().into(),
    }
}
