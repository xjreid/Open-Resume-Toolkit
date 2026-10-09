//! Derive wire types and command signatures from native schemas and Rust syntax.
use serde_json::{Value, json};
use std::{collections::BTreeMap, fmt::Write, fs, path::Path};
use syn::{FnArg, GenericArgument, Item, Pat, PathArguments, ReturnType, Type};

pub fn write(root: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut roots = ort_desktop::desktop_wire_schemas();
    for entry in fs::read_dir(output)? {
        let path = entry?.path();
        if path.to_string_lossy().ends_with(".schema.json")
            && path
                .file_name()
                .is_some_and(|name| name != "wire.schema.json")
        {
            let schema: Value = serde_json::from_slice(&fs::read(path)?)?;
            if let Some(title) = schema["title"].as_str() {
                roots.insert(title.into(), schema);
            }
        }
    }
    let mut definitions = BTreeMap::new();
    for (name, mut schema) in roots {
        if let Some(nested) = schema.get_mut("$defs").and_then(Value::as_object_mut) {
            for (key, value) in std::mem::take(nested) {
                definitions.insert(key, value);
            }
        }
        schema
            .as_object_mut()
            .ok_or("object schema required")?
            .remove("$schema");
        definitions.insert(name, schema);
    }
    let signatures = command_signatures(root)?;
    let schema = json!({"$defs": definitions, "commands": signatures});
    check_refs(&schema, &definitions)?;
    fs::write(
        output.join("wire.schema.json"),
        format!("{}\n", serde_json::to_string_pretty(&schema)?),
    )?;
    let mut types =
        String::from("// @generated from native JSON schemas and command signatures.\n");
    for (name, value) in &definitions {
        writeln!(types, "export type {name} = {};", ts(value))?;
    }
    types.push_str("export type DesktopCommands = {\n");
    for (name, signature) in &signatures {
        writeln!(
            types,
            "  {name}: {{ args: {}; value: {} }};",
            ts(&signature["args"]),
            ts(&signature["value"])
        )?;
    }
    types.push_str("};\n");
    fs::write(output.join("wire.ts"), types)?;
    fs::write(
        output.join("wire-decoder.ts"),
        include_str!("wire-decoder.ts.template"),
    )?;
    fs::write(
        output.join("wire-fixtures.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&ort_desktop::desktop_wire_fixtures())?
        ),
    )?;
    let files = fs::read_dir(output)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    let formatted = std::process::Command::new(root.join("node_modules/.bin/prettier"))
        .arg("--write")
        .args(
            files
                .iter()
                .filter(|path| path.extension().is_some_and(|ext| ext == "ts")),
        )
        .output()?;
    if !formatted.status.success() {
        return Err(
            "contract formatting failed; install workspace development dependencies".into(),
        );
    }
    Ok(())
}

fn command_signatures(root: &Path) -> Result<BTreeMap<String, Value>, Box<dyn std::error::Error>> {
    let mut signatures = BTreeMap::new();
    for module in [
        "ai_keys",
        "chatgpt_plan",
        "codex_readiness",
        "codex_install",
        "ai_settings",
        "ai_request",
        "application_materials",
        "application_exports",
        "tracker",
        "browser_bridge",
        "backup_export",
        "lib",
    ] {
        let source =
            fs::read_to_string(root.join(format!("apps/desktop/src-tauri/src/{module}.rs")))?;
        for item in syn::parse_file(&source)?.items {
            let Item::Fn(function) = item else { continue };
            if !function.attrs.iter().any(|attribute| {
                attribute
                    .path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "command")
            }) {
                continue;
            }
            let ReturnType::Type(_, result) = function.sig.output else {
                continue;
            };
            let Type::Path(result) = *result else {
                continue;
            };
            let Some(segment) = result.path.segments.last() else {
                continue;
            };
            if segment.ident != "CommandResponse" {
                continue;
            }
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                continue;
            };
            let Some(GenericArgument::Type(value)) = arguments.args.first() else {
                continue;
            };
            let mut properties = BTreeMap::new();
            let mut required = Vec::new();
            for arg in function.sig.inputs {
                let FnArg::Typed(arg) = arg else { continue };
                let Pat::Ident(pattern) = *arg.pat else {
                    continue;
                };
                if matches!(
                    path_name(&arg.ty).as_str(),
                    "State" | "WebviewWindow" | "AppHandle"
                ) {
                    continue;
                }
                let name = camel(&pattern.ident.to_string());
                if path_name(&arg.ty) != "Option" {
                    required.push(name.clone());
                }
                properties.insert(name, rust_schema(&arg.ty));
            }
            signatures.insert(function.sig.ident.to_string(), json!({"args":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"value":rust_schema(value)}));
        }
    }
    Ok(signatures)
}

fn check_refs(
    value: &Value,
    definitions: &BTreeMap<String, Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    match value {
        Value::Object(values) => {
            if let Some(reference) = values.get("$ref").and_then(Value::as_str) {
                let name = reference
                    .strip_prefix("#/$defs/")
                    .ok_or("unsupported schema reference")?;
                if !definitions.contains_key(name) {
                    return Err(format!("missing wire schema: {name}").into());
                }
            }
            for value in values.values() {
                check_refs(value, definitions)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                check_refs(value, definitions)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn camel(name: &str) -> String {
    let mut pieces = name.split('_');
    let mut result = pieces.next().unwrap_or_default().to_owned();
    for piece in pieces {
        let mut chars = piece.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.push_str(chars.as_str());
        }
    }
    result
}
fn path_name(ty: &Type) -> String {
    match ty {
        Type::Reference(reference) => path_name(&reference.elem),
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}
fn rust_schema(ty: &Type) -> Value {
    if let Type::Reference(reference) = ty {
        return rust_schema(&reference.elem);
    }
    let Type::Path(path) = ty else {
        return json!({});
    };
    let Some(segment) = path.path.segments.last() else {
        return json!({});
    };
    let name = segment.ident.to_string();
    match name.as_str() {
        "String" | "str" | "Uuid" => json!({"type":"string"}),
        "bool" => json!({"type":"boolean"}),
        "i64" | "u64" | "u32" | "i32" | "u16" | "usize" => json!({"type":"integer"}),
        "f64" => json!({"type":"number"}),
        "Value" | "Channel" => json!({}),
        "Vec" | "Option" => {
            let PathArguments::AngleBracketed(args) = &segment.arguments else {
                return json!({});
            };
            let Some(GenericArgument::Type(inner)) = args.args.first() else {
                return json!({});
            };
            if name == "Vec" {
                json!({"type":"array","items":rust_schema(inner)})
            } else {
                json!({"anyOf":[rust_schema(inner),{"type":"null"}]})
            }
        }
        _ => json!({"$ref":format!("#/$defs/{name}")}),
    }
}
fn ts(schema: &Value) -> String {
    if let Some(reference) = schema["$ref"].as_str() {
        return reference.rsplit('/').next().unwrap_or("never").into();
    }
    if let Some(value) = schema.get("const") {
        return value.to_string();
    }
    if let Some(values) = schema["enum"].as_array() {
        return values
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join(" | ");
    }
    for (key, join) in [("anyOf", " | "), ("oneOf", " | "), ("allOf", " & ")] {
        if let Some(values) = schema[key].as_array() {
            return format!("({})", values.iter().map(ts).collect::<Vec<_>>().join(join));
        }
    }
    if let Some(values) = schema["type"].as_array() {
        return values
            .iter()
            .map(|value| ts(&json!({"type":value})))
            .collect::<Vec<_>>()
            .join(" | ");
    }
    match schema["type"].as_str() {
        Some("string") => "string".into(),
        Some("integer" | "number") => "number".into(),
        Some("boolean") => "boolean".into(),
        Some("null") => "null".into(),
        Some("array") => format!("Array<{}>", ts(&schema["items"])),
        Some("object") => {
            let Some(properties) = schema["properties"].as_object() else {
                return format!("Record<string, {}>", ts(&schema["additionalProperties"]));
            };
            if properties.is_empty() {
                return if schema["additionalProperties"] == Value::Bool(false) {
                    "Record<string, never>".into()
                } else {
                    "Record<string, unknown>".into()
                };
            }
            let required = schema["required"].as_array();
            let fields = properties
                .iter()
                .map(|(name, value)| {
                    let optional = if required.is_some_and(|required| {
                        required.iter().any(|item| item.as_str() == Some(name))
                    }) {
                        ""
                    } else {
                        "?"
                    };
                    format!("{name}{optional}: {}", ts(value))
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("{{ {fields} }}")
        }
        _ => {
            if schema == &Value::Bool(false) {
                "never".into()
            } else {
                "unknown".into()
            }
        }
    }
}
