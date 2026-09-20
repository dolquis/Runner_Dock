//! 契約生成物（JSON Schema と TypeScript 型）を `crates/protocol` から作る。
//!
//! `docs/04_TECHNOLOGY_STACK.md` §4 の「serde DTO から JSON Schema / TypeScript を
//! 生成する薄いツール」にあたる。ワイヤー契約を外部の型生成ツールへ固定しないため、
//! schemars の出力を自前で TypeScript へ落とす。扱う schema の形は、この crate が
//! 実際に出すもの（object、文字列 enum、`$ref`、配列、`Option` の anyOf）に限る。
//!
//! 使い方:
//!   `cargo run -p runnerdock-protocol --bin contracts-gen`          生成物を書く
//!   `cargo run -p runnerdock-protocol --bin contracts-gen -- --check` 差分だけ見る

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use schemars::{JsonSchema, schema_for};
use serde_json::{Map, Value};

use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::{
    CredentialRefView, ForceStopRequest, HandshakeResult, NodeOperationRequest, NodeSnapshot,
    OperationAccepted, OperationSnapshot,
};
use runnerdock_protocol::error::ErrorPayload;
use runnerdock_protocol::message::{EventKind, Message};
use runnerdock_protocol::method::Method;

/// 生成物へ載せる型の集合。ここに載せた型だけが UI から参照できる。
///
/// フィールドは schema を集めるためだけのもので、値としては構築しない。
#[derive(JsonSchema)]
#[expect(dead_code, reason = "schema 収集専用。実行時には構築しない")]
struct ContractSurface {
    message: Message,
    method: Method,
    event_kind: EventKind,
    node_snapshot: NodeSnapshot,
    operation_snapshot: OperationSnapshot,
    error_payload: ErrorPayload,
    credential_ref: CredentialRefView,
    handshake_result: HandshakeResult,
    node_operation_request: NodeOperationRequest,
    force_stop_request: ForceStopRequest,
    operation_accepted: OperationAccepted,
}

const SCHEMA_PATH: &str = "packages/contracts/schema/protocol.schema.json";
const TYPES_PATH: &str = "packages/contracts/src/protocol.ts";

fn main() -> ExitCode {
    let check_only = std::env::args().any(|arg| arg == "--check");
    let root = match workspace_root() {
        Some(root) => root,
        None => {
            eprintln!("workspace のルートが見つからない");
            return ExitCode::FAILURE;
        }
    };

    let schema = build_schema();
    let schema_text = match serde_json::to_string_pretty(&schema) {
        Ok(mut text) => {
            text.push('\n');
            text
        }
        Err(error) => {
            eprintln!("schema の直列化に失敗した: {error}");
            return ExitCode::FAILURE;
        }
    };
    let types_text = render_typescript(&schema);

    let artifacts = [
        (root.join(SCHEMA_PATH), schema_text, SCHEMA_PATH),
        (root.join(TYPES_PATH), types_text, TYPES_PATH),
    ];

    let mut drifted = Vec::new();
    for (path, content, label) in &artifacts {
        let current = std::fs::read_to_string(path).ok();
        if current.as_deref() == Some(content.as_str()) {
            continue;
        }
        drifted.push(*label);
        if !check_only && let Err(error) = write_artifact(path, content) {
            eprintln!("{label} を書けない: {error}");
            return ExitCode::FAILURE;
        }
    }

    if drifted.is_empty() {
        println!("契約生成物は protocol crate と一致している");
        return ExitCode::SUCCESS;
    }

    if check_only {
        eprintln!("契約生成物が protocol crate とずれている:");
        for label in &drifted {
            eprintln!("  - {label}");
        }
        eprintln!(
            "`cargo run -p runnerdock-protocol --bin contracts-gen` を実行して commit すること"
        );
        return ExitCode::FAILURE;
    }

    for label in &drifted {
        println!("更新: {label}");
    }
    ExitCode::SUCCESS
}

fn write_artifact(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

/// このバイナリの位置から workspace のルートを辿る。
fn workspace_root() -> Option<PathBuf> {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join("Cargo.toml").exists() && current.join("crates").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn build_schema() -> Value {
    let mut schema = schema_for!(ContractSurface).to_value();
    if let Value::Object(map) = &mut schema {
        // ContractSurface 自身は収集用の入れ物なので、根の object 定義は落とす。
        map.remove("properties");
        map.remove("required");
        map.remove("type");
        map.remove("title");
        map.insert(
            "title".to_owned(),
            Value::String("Runner Dock IPC contract".to_owned()),
        );
        map.insert(
            "x-protocolMajor".to_owned(),
            Value::Number(PROTOCOL_MAJOR.into()),
        );
    }
    schema
}

// ---- TypeScript の出力 -------------------------------------------------------

fn render_typescript(schema: &Value) -> String {
    let defs = schema
        .get("$defs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str("// このファイルは crates/protocol から生成する。手で編集しない。\n");
    out.push_str(
        "// 再生成: cargo run -p runnerdock-protocol --bin contracts-gen\n\
         // 差分検査: cargo run -p runnerdock-protocol --bin contracts-gen -- --check\n\n",
    );
    out.push_str(&format!(
        "/** ワイヤー契約の major。不一致なら接続しない。 */\nexport const PROTOCOL_MAJOR = {PROTOCOL_MAJOR};\n",
    ));

    // 名前順に出して、生成のたびに並びが揺れないようにする。
    let ordered: BTreeMap<&String, &Value> = defs.iter().collect();
    for (name, definition) in ordered {
        out.push('\n');
        out.push_str(&render_definition(name, definition));
    }
    out
}

fn render_definition(name: &str, definition: &Value) -> String {
    let mut out = String::new();
    if let Some(description) = definition.get("description").and_then(Value::as_str) {
        out.push_str(&render_doc_comment(description, ""));
    }

    if let Some(object) = definition.as_object()
        && object.contains_key("properties")
    {
        out.push_str(&format!("export interface {name} {{\n"));
        out.push_str(&render_properties(object));
        out.push_str("}\n");
        return out;
    }

    out.push_str(&format!("export type {name} = {};\n", type_of(definition)));
    out
}

fn render_properties(object: &Map<String, Value>) -> String {
    let required: Vec<&str> = object
        .get("required")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut out = String::new();
    for (field, schema) in &properties {
        if let Some(description) = schema.get("description").and_then(Value::as_str) {
            out.push_str(&render_doc_comment(description, "  "));
        }
        let optional = if required.contains(&field.as_str()) {
            ""
        } else {
            "?"
        };
        out.push_str(&format!(
            "  readonly {field}{optional}: {};\n",
            type_of(schema)
        ));
    }
    out
}

fn render_doc_comment(description: &str, indent: &str) -> String {
    // 空行を ` * ` のまま出すと行末に空白が残り、trailing-whitespace の整形で
    // 生成物が書き換わって差分検出が誤検知する。
    let body = description
        .lines()
        .map(|line| {
            let line = line.trim_end();
            if line.is_empty() {
                format!("{indent} *\n")
            } else {
                format!("{indent} * {line}\n")
            }
        })
        .collect::<String>();
    format!("{indent}/**\n{body}{indent} */\n")
}

/// schema の 1 ノードを TypeScript の型式へ変換する。
fn type_of(schema: &Value) -> String {
    let Some(object) = schema.as_object() else {
        // `true` / `false` schema。制約なしとして扱う。
        return "unknown".to_owned();
    };

    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        let name = reference.rsplit('/').next().unwrap_or("unknown").to_owned();
        // 内部タグ付き enum の変種は `$ref` と判別用 `kind` を同時に持つ。交差型に
        // しないと TypeScript 側で共用体を判別できなくなる。
        if object.contains_key("properties") {
            let body = render_properties(object);
            return format!("{name} & {{\n{body}}}");
        }
        return name;
    }

    if let Some(constant) = object.get("const") {
        return literal_of(constant);
    }

    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        return union_of(values.iter().map(literal_of));
    }

    for key in ["oneOf", "anyOf"] {
        if let Some(variants) = object.get(key).and_then(Value::as_array) {
            return union_of(variants.iter().map(type_of));
        }
    }

    match object.get("type") {
        Some(Value::String(kind)) => primitive_of(kind, object),
        // `["string","null"]` のような複合型。
        Some(Value::Array(kinds)) => union_of(
            kinds
                .iter()
                .filter_map(Value::as_str)
                .map(|kind| primitive_of(kind, object)),
        ),
        _ => "unknown".to_owned(),
    }
}

fn primitive_of(kind: &str, object: &Map<String, Value>) -> String {
    match kind {
        "string" => "string".to_owned(),
        "boolean" => "boolean".to_owned(),
        "integer" | "number" => "number".to_owned(),
        "null" => "null".to_owned(),
        "array" => {
            let item = object.get("items").map_or_else(
                || "unknown".to_owned(),
                |items| parenthesize(&type_of(items)),
            );
            format!("readonly {item}[]")
        }
        "object" => {
            if object.contains_key("properties") {
                let body = render_properties(object);
                format!("{{\n{body}}}")
            } else {
                let value = object
                    .get("additionalProperties")
                    .map_or_else(|| "unknown".to_owned(), type_of);
                format!("{{ readonly [key: string]: {value} }}")
            }
        }
        _ => "unknown".to_owned(),
    }
}

fn literal_of(value: &Value) -> String {
    match value {
        Value::String(text) => format!("\"{text}\""),
        Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

fn union_of(parts: impl Iterator<Item = String>) -> String {
    let mut seen: Vec<String> = Vec::new();
    for part in parts {
        if !seen.contains(&part) {
            seen.push(part);
        }
    }
    if seen.is_empty() {
        return "unknown".to_owned();
    }
    seen.join(" | ")
}

/// 共用体を配列要素などへ入れるときに括弧を足す。
fn parenthesize(type_expression: &str) -> String {
    if type_expression.contains(" | ") {
        format!("({type_expression})")
    } else {
        type_expression.to_owned()
    }
}
