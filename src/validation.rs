use once_cell::sync::Lazy;
use serde_json::Value;

use jsonschema::error::ValidationErrorKind;
use jsonschema::{Draft, JSONSchema};

use crate::model::ValidationIssue;

static INVOCATION_SCHEMA: Lazy<JSONSchema> = Lazy::new(|| {
    let schema: Value = serde_json::from_str(include_str!(
        "../schemas/adaptive-card.invocation.v1.schema.json"
    ))
    .expect("invocation schema JSON must be valid");
    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema)
        .expect("invocation schema must compile")
});

pub fn locate_invocation_candidate(value: &Value) -> Option<Value> {
    if let Some(inv) = find_invocation_value(value) {
        return Some(inv);
    }
    if let Some(payload) = value.get("payload")
        && payload.is_object()
    {
        return Some(payload.clone());
    }
    if let Some(config) = value.get("config")
        && config.is_object()
    {
        return Some(config.clone());
    }
    None
}

pub fn validate_invocation_schema(value: &Value) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let result = INVOCATION_SCHEMA.validate(value);
    if let Err(errors) = result {
        for error in errors {
            issues.push(map_schema_error(&error));
        }
    }
    issues
}

fn map_schema_error(error: &jsonschema::ValidationError) -> ValidationIssue {
    let code = match error.kind {
        ValidationErrorKind::Required { .. } => "AC_INVOCATION_MISSING_FIELD",
        ValidationErrorKind::Type { .. } => "AC_INVOCATION_INVALID_TYPE",
        ValidationErrorKind::Enum { .. } => "AC_INVOCATION_INVALID_ENUM",
        _ => "AC_INVOCATION_SCHEMA_ERROR",
    };
    let raw_path = error.instance_path.to_string();
    let path = if raw_path.is_empty() {
        "/".to_string()
    } else {
        raw_path
    };
    ValidationIssue {
        code: code.to_string(),
        message: error.to_string(),
        path,
    }
}

fn find_invocation_value(value: &Value) -> Option<Value> {
    let obj = value.as_object()?;
    if obj.contains_key("card_source") || obj.contains_key("card_spec") {
        return Some(value.clone());
    }
    if let Some(inv) = obj.get("invocation") {
        return Some(inv.clone());
    }
    if let Some(card) = obj.get("card") {
        return Some(card.clone());
    }
    if let Some(payload) = obj.get("payload")
        && payload
            .as_object()
            .map(|p| p.contains_key("card_source") || p.contains_key("card_spec"))
            .unwrap_or(false)
    {
        return Some(payload.clone());
    }
    if let Some(config) = obj.get("config") {
        if config
            .as_object()
            .map(|c| c.contains_key("card_source") || c.contains_key("card_spec"))
            .unwrap_or(false)
        {
            return Some(config.clone());
        }
        if let Some(card) = config.get("card") {
            return Some(card.clone());
        }
    }
    None
}
