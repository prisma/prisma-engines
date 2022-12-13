mod error;
mod rust_crate;

use self::error::CrateResult;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

pub fn generate_rust_modules(out_dir: &Path) -> CrateResult {
    let api_defs_root = concat!(env!("CARGO_MANIFEST_DIR"), "/methods");

    // https://doc.rust-lang.org/cargo/reference/build-scripts.html
    println!("cargo:rerun-if-changed={}", api_defs_root);

    let entries = std::fs::read_dir(api_defs_root)?;
    let mut api = Api::default();

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let contents = std::fs::read_to_string(entry.path())?;
        eprintln!("Merging {}", entry.path().file_name().unwrap().to_string_lossy());
        let api_fragment: Api = toml::from_str(&contents)?;

        merge(&mut api, api_fragment);
    }

    validate(&api);

    rust_crate::generate_rust_crate(out_dir, &api)?;

    eprintln!("ok: definitions generated");

    Ok(())
}

fn validate(api: &Api) {
    let mut errs: Vec<String> = Vec::new();

    for (method_name, method) in &api.methods {
        if !shape_exists(&method.request_shape, api) {
            errs.push(format!("Request shape for {} does not exist", method_name))
        }

        if !shape_exists(&method.response_shape, api) {
            errs.push(format!("Response shape for {} does not exist", method_name))
        }
    }

    for (record_name, record_shape) in &api.record_shapes {
        for (field_name, field) in &record_shape.fields {
            if !shape_exists(&field.shape, api) {
                errs.push(format!(
                    "Field shape for {}.{} does not exist.",
                    record_name, field_name
                ))
            }
        }
    }

    for (enum_name, enum_shape) in &api.enum_shapes {
        for (variant_name, variant) in &enum_shape.variants {
            if let Some(shape) = variant.shape.as_ref() {
                if !shape_exists(shape, api) {
                    errs.push(format!(
                        "Enum variant shape for {}.{} does not exist.",
                        enum_name, variant_name
                    ))
                }
            }
        }
    }

    if !errs.is_empty() {
        for err in errs {
            eprintln!("{}", err);
        }
        std::process::exit(1);
    }
}

fn shape_exists(shape: &str, api: &Api) -> bool {
    let builtin_scalars = ["string", "bool", "u32", "isize", "serde_json::Value"];

    if builtin_scalars.contains(&shape) {
        return true;
    }

    if api.enum_shapes.contains_key(shape) {
        return true;
    }

    if api.record_shapes.contains_key(shape) {
        return true;
    }

    false
}

fn merge(api: &mut Api, new_fragment: Api) {
    for (method_name, method) in new_fragment.methods {
        assert!(api.methods.insert(method_name, method).is_none());
    }

    for (record_name, record) in new_fragment.record_shapes {
        assert!(api.record_shapes.insert(record_name, record).is_none());
    }

    for (enum_name, enum_d) in new_fragment.enum_shapes {
        assert!(api.enum_shapes.insert(enum_name, enum_d).is_none());
    }
}

// Make sure #[serde(deny_unknown_fields)] is on all struct types here.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Api {
    #[serde(rename = "recordShapes", default)]
    record_shapes: HashMap<String, RecordShape>,
    #[serde(rename = "enumShapes", default)]
    enum_shapes: HashMap<String, EnumShape>,
    #[serde(default)]
    methods: HashMap<String, Method>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordShape {
    description: Option<String>,
    #[serde(default)]
    fields: BTreeMap<String, RecordField>,
    example: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordField {
    description: Option<String>,
    #[serde(rename = "isList", default)]
    is_list: bool,
    #[serde(rename = "isNullable", default)]
    is_nullable: bool,
    shape: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnumVariant {
    description: Option<String>,
    /// In case there is no shape, it just means the variant has no associated data.
    shape: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnumShape {
    description: Option<String>,
    variants: HashMap<String, EnumVariant>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Method {
    description: Option<String>,
    #[serde(rename = "requestShape")]
    request_shape: String,
    #[serde(rename = "responseShape")]
    response_shape: String,
}
