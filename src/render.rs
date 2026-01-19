use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use handlebars::Handlebars;
use serde_json::{Map, Value};

use crate::asset_resolver::resolve_with_host;
use crate::error::ComponentError;
use crate::expression::{ExpressionEngine, SimpleExpressionEngine, stringify_value};
use crate::model::{
    AdaptiveCardInvocation, CardFeatureSummary, CardSource, CardSpec, ValidationIssue,
};

#[derive(Debug)]
pub struct RenderOutcome {
    pub card: Value,
    pub features: CardFeatureSummary,
    pub validation_issues: Vec<ValidationIssue>,
}

pub fn render_card(inv: &AdaptiveCardInvocation) -> Result<RenderOutcome, ComponentError> {
    let mut card = resolve_card(inv)?;
    apply_handlebars(&mut card, inv)?;
    let ctx = BindingContext::from_invocation(inv);
    let engine = SimpleExpressionEngine;
    apply_bindings(&mut card, &ctx, &engine);

    let features = analyze_features(&card);
    let validation_issues = validate_card(&card);

    Ok(RenderOutcome {
        card,
        features,
        validation_issues,
    })
}

fn resolve_card(inv: &AdaptiveCardInvocation) -> Result<Value, ComponentError> {
    match inv.card_source {
        CardSource::Inline => inv
            .card_spec
            .inline_json
            .clone()
            .ok_or_else(|| ComponentError::InvalidInput("inline_json is required".into())),
        CardSource::Asset => {
            let path = inv
                .card_spec
                .asset_path
                .as_ref()
                .ok_or_else(|| ComponentError::InvalidInput("asset_path is required".into()))?;
            let candidates = candidate_asset_paths(path, inv.card_spec.asset_registry.as_ref())?;
            load_with_candidates(path, candidates)
        }
        CardSource::Catalog => {
            let catalog =
                inv.card_spec.catalog_name.as_ref().ok_or_else(|| {
                    ComponentError::InvalidInput("catalog_name is required".into())
                })?;
            let normalized = catalog.trim_start_matches('/');
            let candidates = candidate_catalog_paths(normalized, &inv.card_spec)?;
            load_with_candidates(normalized, candidates)
        }
    }
}

fn resolve_catalog_mapping(name: &str, spec: &CardSpec) -> Result<Option<String>, ComponentError> {
    if let Some(registry) = spec.asset_registry.as_ref()
        && let Some(path) = registry.get(name)
    {
        return Ok(Some(path.to_string()));
    }
    if let Some(env_registry) = env_asset_registry()?
        && let Some(path) = env_registry.get(name)
    {
        return Ok(Some(path.to_string()));
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        return Ok(None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = match std::env::var("ADAPTIVE_CARD_CATALOG_FILE") {
            Ok(path) => path,
            Err(_) => return Ok(None),
        };
        let content = std::fs::read_to_string(file)?;
        let map: BTreeMap<String, String> = serde_json::from_str(&content)?;
        Ok(map.get(name).cloned())
    }
}

fn env_asset_registry() -> Result<Option<BTreeMap<String, String>>, ComponentError> {
    #[cfg(target_arch = "wasm32")]
    {
        return Ok(None);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = match std::env::var("ADAPTIVE_CARD_ASSET_REGISTRY") {
            Ok(path) => path,
            Err(_) => return Ok(None),
        };
        let content = std::fs::read_to_string(file)?;
        let map: BTreeMap<String, String> = serde_json::from_str(&content)?;
        Ok(Some(map))
    }
}

fn candidate_asset_paths(
    path: &str,
    registry: Option<&BTreeMap<String, String>>,
) -> Result<Vec<String>, ComponentError> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let push = |value: String, seen: &mut HashSet<String>, list: &mut Vec<String>| {
        if seen.insert(value.clone()) {
            list.push(value);
        }
    };

    if let Some(registry) = registry
        && let Some(mapped) = registry.get(path)
    {
        push(mapped.to_string(), &mut seen, &mut candidates);
    }

    if let Ok(Some(env_map)) = env_asset_registry()
        && let Some(mapped) = env_map.get(path)
    {
        push(mapped.to_string(), &mut seen, &mut candidates);
    }

    if Path::new(path).is_absolute()
        || path.starts_with("./")
        || path.starts_with("../")
        || path.contains('/')
    {
        push(path.to_string(), &mut seen, &mut candidates);
    } else {
        let base = asset_base_path();
        let joined = PathBuf::from(base).join(path).to_string_lossy().to_string();
        push(joined, &mut seen, &mut candidates);
        push(path.to_string(), &mut seen, &mut candidates);
    }

    Ok(candidates)
}

fn candidate_catalog_paths(name: &str, spec: &CardSpec) -> Result<Vec<String>, ComponentError> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let push = |value: String, seen: &mut HashSet<String>, list: &mut Vec<String>| {
        if seen.insert(value.clone()) {
            list.push(value);
        }
    };

    if let Some(mapped) = resolve_catalog_mapping(name, spec)? {
        push(mapped, &mut seen, &mut candidates);
    }

    let base = asset_base_path();
    let path = format!("{}/{}.json", base, name);
    push(path, &mut seen, &mut candidates);
    if Path::new(name).is_absolute() || name.contains('/') || name.ends_with(".json") {
        push(name.to_string(), &mut seen, &mut candidates);
    }

    Ok(candidates)
}

fn asset_base_path() -> String {
    std::env::var("ADAPTIVE_CARD_ASSET_BASE").unwrap_or_else(|_| "assets".to_string())
}

fn load_card_from_path(path: &str) -> Result<Value, ComponentError> {
    let content = std::fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&content)?;
    Ok(json)
}

fn load_with_candidates(
    lookup_key: &str,
    candidates: Vec<String>,
) -> Result<Value, ComponentError> {
    let mut last_err: Option<ComponentError> = None;
    for candidate in candidates {
        match load_card_from_path(&candidate) {
            Ok(card) => return Ok(card),
            Err(err) => last_err = Some(err),
        }
    }

    if let Some(host) =
        resolve_with_host(lookup_key).map_err(|e| ComponentError::Asset(e.message))?
    {
        match load_card_from_path(&host) {
            Ok(card) => return Ok(card),
            Err(err) => last_err = Some(err),
        }
    }

    Err(last_err.unwrap_or_else(|| {
        ComponentError::InvalidInput(format!("unable to resolve card for {lookup_key}"))
    }))
}

#[derive(Debug)]
pub struct BindingContext {
    payload: Value,
    session: Value,
    state: Value,
    template_params: Value,
}

impl BindingContext {
    fn from_invocation(inv: &AdaptiveCardInvocation) -> Self {
        BindingContext {
            payload: inv.payload.clone(),
            session: inv.session.clone(),
            state: inv.state.clone(),
            template_params: inv
                .card_spec
                .template_params
                .clone()
                .unwrap_or(Value::Object(Map::new())),
        }
    }

    pub fn lookup(&self, raw: &str) -> Option<Value> {
        let (path, default) = parse_binding_path(raw);
        let mut segments = path.split('.');
        let first = segments.next()?;
        let attempt_root = |root: &Value, rest: std::str::Split<'_, char>| lookup_in(root, rest);

        let found = match first {
            "payload" => attempt_root(&self.payload, segments),
            "session" => attempt_root(&self.session, segments),
            "state" => attempt_root(&self.state, segments),
            "params" | "template" => attempt_root(&self.template_params, segments),
            _ => lookup_in(
                &self.payload,
                normalize_path(&path)
                    .split('.')
                    .collect::<Vec<_>>()
                    .into_iter(),
            )
            .or_else(|| {
                lookup_in(
                    &self.session,
                    normalize_path(&path)
                        .split('.')
                        .collect::<Vec<_>>()
                        .into_iter(),
                )
            })
            .or_else(|| {
                lookup_in(
                    &self.state,
                    normalize_path(&path)
                        .split('.')
                        .collect::<Vec<_>>()
                        .into_iter(),
                )
            })
            .or_else(|| {
                lookup_in(
                    &self.template_params,
                    normalize_path(&path)
                        .split('.')
                        .collect::<Vec<_>>()
                        .into_iter(),
                )
            }),
        };

        match (found, default) {
            (Some(value), _) if !value.is_null() => Some(value),
            (None, Some(fallback)) | (Some(Value::Null), Some(fallback)) => Some(fallback),
            (other, _) => other,
        }
    }
}

fn lookup_in<'a, I>(value: &Value, mut parts: I) -> Option<Value>
where
    I: Iterator<Item = &'a str>,
{
    let mut current = value;
    for part in parts.by_ref() {
        match current {
            Value::Object(map) => current = map.get(part)?,
            Value::Array(items) => {
                let idx: usize = part.parse().ok()?;
                current = items.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

fn apply_bindings(value: &mut Value, ctx: &BindingContext, engine: &dyn ExpressionEngine) {
    match value {
        Value::String(text) => {
            if let Some(path) = extract_single_placeholder(text)
                && let Some(resolved) = ctx.lookup(path)
            {
                *value = resolved;
                return;
            }
            if let Some(expr) = extract_expression(text)
                && let Some(resolved) = engine.eval(expr, ctx)
            {
                *value = match resolved {
                    Value::String(_) => resolved,
                    other => Value::String(stringify_value(&other)),
                };
                return;
            }
            let replaced = replace_placeholders(text, ctx);
            *value = Value::String(replaced);
        }
        Value::Array(items) => {
            for item in items {
                apply_bindings(item, ctx, engine);
            }
        }
        Value::Object(map) => {
            for entry in map.values_mut() {
                apply_bindings(entry, ctx, engine);
            }
        }
        _ => {}
    }
}

fn apply_handlebars(value: &mut Value, inv: &AdaptiveCardInvocation) -> Result<(), ComponentError> {
    let mut engine = Handlebars::new();
    engine.set_strict_mode(false);
    let context = build_handlebars_context(inv);
    render_handlebars_value(value, &engine, &context)
}

fn render_handlebars_value(
    value: &mut Value,
    engine: &Handlebars<'_>,
    context: &Value,
) -> Result<(), ComponentError> {
    match value {
        Value::String(text) => {
            let rendered = engine
                .render_template(text, context)
                .map_err(|err| ComponentError::InvalidInput(format!("handlebars: {err}")))?;
            *value = Value::String(rendered);
            Ok(())
        }
        Value::Array(items) => {
            for item in items {
                render_handlebars_value(item, engine, context)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for entry in map.values_mut() {
                render_handlebars_value(entry, engine, context)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn build_handlebars_context(inv: &AdaptiveCardInvocation) -> Value {
    let mut root = Map::new();
    root.insert("payload".to_owned(), inv.payload.clone());
    root.insert("state".to_owned(), inv.state.clone());

    if let Some(node_id) = inv.node_id.as_deref() {
        root.insert("node_id".to_owned(), Value::String(node_id.to_owned()));
        if let Some(node) = resolve_state_node(&inv.state, node_id) {
            if let Some(payload) = node.get("payload") {
                root.insert("node_payload".to_owned(), payload.clone());
            }
            root.insert("node".to_owned(), Value::Object(node));
        }
    }

    if let Some(state_input) = resolve_state_input(&inv.state) {
        for (key, value) in state_input {
            if is_reserved_handlebars_key(&key) || root.contains_key(&key) {
                continue;
            }
            root.insert(key, value);
        }
    }

    Value::Object(root)
}

fn resolve_state_node(state: &Value, node_id: &str) -> Option<Map<String, Value>> {
    let nodes = state.get("nodes")?.as_object()?;
    let node = nodes.get(node_id)?.as_object()?;
    Some(node.clone())
}

fn resolve_state_input(state: &Value) -> Option<Map<String, Value>> {
    state.get("input")?.as_object().cloned()
}

fn is_reserved_handlebars_key(key: &str) -> bool {
    matches!(
        key,
        "payload" | "state" | "node" | "node_id" | "node_payload"
    )
}

fn replace_placeholders(input: &str, ctx: &BindingContext) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let bytes = input.as_bytes();
    while cursor < input.len() {
        let remaining = &input[cursor..];
        let next_at = remaining.find("@{");
        let next_dollar = remaining.find("${");
        let next_pos = match (next_at, next_dollar) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let Some(pos) = next_pos else {
            output.push_str(&input[cursor..]);
            break;
        };

        let absolute = cursor + pos;
        output.push_str(&input[cursor..absolute]);

        let marker = input.as_bytes()[absolute];
        if absolute + 2 > input.len() || bytes[absolute + 1] != b'{' {
            output.push_str(&input[absolute..]);
            break;
        }
        let rest = &input[absolute + 2..];
        if let Some(end) = rest.find('}') {
            let path = &rest[..end];
            let replacement = ctx.lookup(path.trim()).map(|v| match v {
                Value::String(s) => s,
                other => other.to_string(),
            });
            let replacement = replacement.unwrap_or_default();
            output.push_str(&replacement);
            cursor = absolute + 2 + end + 1;
        } else {
            output.push(marker as char);
            cursor = absolute + 1;
        }
    }

    output
}

fn extract_single_placeholder(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if let Some(stripped) = trimmed.strip_prefix("@{").and_then(|s| s.strip_suffix('}')) {
        return Some(stripped.trim());
    }
    if let Some(stripped) = trimmed.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return Some(stripped.trim());
    }
    None
}

fn parse_binding_path(raw: &str) -> (String, Option<Value>) {
    let mut parts = raw.splitn(2, "||");
    let path = parts.next().unwrap_or(raw).trim().to_string();
    let default = parts.next().and_then(|d| {
        let trimmed = d.trim();
        if trimmed.is_empty() {
            return None;
        }
        serde_json::from_str::<Value>(trimmed)
            .ok()
            .or_else(|| Some(Value::String(trimmed.to_string())))
    });
    (path, default)
}

fn extract_expression(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if let Some(stripped) = trimmed.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return Some(stripped.trim());
    }
    None
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.replace('[', ".").replace(']', "");
    normalized = normalized.replace("..", ".");
    normalized.trim_matches('.').to_string()
}

pub fn analyze_features(card: &Value) -> CardFeatureSummary {
    let mut used_elements = BTreeSet::new();
    let mut used_actions = BTreeSet::new();
    let mut summary = CardFeatureSummary {
        version: card
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        ..CardFeatureSummary::default()
    };

    fn merge_requires(target: &mut Value, new_value: &Value) {
        match target {
            Value::Object(dst) => {
                if let Value::Object(src) = new_value {
                    for (k, v) in src {
                        dst.entry(k.clone()).or_insert(v.clone());
                    }
                }
            }
            Value::Null => *target = new_value.clone(),
            _ => {}
        }
    }

    fn walk(
        value: &Value,
        used_elements: &mut BTreeSet<String>,
        used_actions: &mut BTreeSet<String>,
        summary: &mut CardFeatureSummary,
    ) {
        match value {
            Value::Object(map) => {
                if let Some(kind) = map.get("type").and_then(|v| v.as_str()) {
                    if kind.starts_with("Action.") {
                        used_actions.insert(kind.to_string());
                        if kind == "Action.ShowCard" {
                            summary.uses_show_card = true;
                        }
                        if kind == "Action.ToggleVisibility" {
                            summary.uses_toggle_visibility = true;
                        }
                    } else {
                        used_elements.insert(kind.to_string());
                        if kind == "Media" {
                            summary.uses_media = true;
                        }
                    }
                }
                if map.contains_key("authentication") {
                    summary.uses_auth = true;
                }
                if let Some(req) = map.get("requires") {
                    merge_requires(&mut summary.requires_features, req);
                }
                for value in map.values() {
                    walk(value, used_elements, used_actions, summary);
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, used_elements, used_actions, summary);
                }
            }
            _ => {}
        }
    }

    walk(card, &mut used_elements, &mut used_actions, &mut summary);
    summary.used_elements = used_elements.into_iter().collect();
    summary.used_actions = used_actions.into_iter().collect();
    summary
}

pub fn validate_card(card: &Value) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if !card.is_object() {
        issues.push(ValidationIssue {
            code: "invalid-root".into(),
            message: "Card must be a JSON object".into(),
            path: "/".into(),
        });
        return issues;
    }

    let type_value = card.get("type").and_then(|v| v.as_str());
    if type_value != Some("AdaptiveCard") {
        issues.push(ValidationIssue {
            code: "invalid-type".into(),
            message: "Root type must be AdaptiveCard".into(),
            path: "/type".into(),
        });
    }
    if card.get("version").is_none() {
        issues.push(ValidationIssue {
            code: "missing-version".into(),
            message: "AdaptiveCard must include a version".into(),
            path: "/version".into(),
        });
    }

    let mut input_ids = HashSet::new();

    fn push_issue(path: &str, code: &str, message: &str, issues: &mut Vec<ValidationIssue>) {
        issues.push(ValidationIssue {
            code: code.to_string(),
            message: message.to_string(),
            path: path.to_string(),
        });
    }

    fn visit(
        value: &Value,
        path: &str,
        issues: &mut Vec<ValidationIssue>,
        input_ids: &mut HashSet<String>,
        action_ids: &mut HashSet<String>,
    ) {
        match value {
            Value::Object(map) => {
                let kind = map.get("type").and_then(|v| v.as_str()).unwrap_or_default();
                if kind.starts_with("Input.") && !map.contains_key("id") {
                    push_issue(path, "missing-id", "Inputs must include an id", issues);
                }
                if kind.starts_with("Input.")
                    && let Some(id) = map.get("id").and_then(|v| v.as_str())
                {
                    let inserted = input_ids.insert(id.to_string());
                    if !inserted {
                        push_issue(
                            path,
                            "duplicate-id",
                            "Input ids should be unique within the card",
                            issues,
                        );
                    }
                }
                if kind.starts_with("Action.") {
                    if let Some(id) = map.get("id").and_then(|v| v.as_str())
                        && !action_ids.insert(id.to_string())
                    {
                        push_issue(
                            path,
                            "duplicate-action-id",
                            "Action ids should be unique within the card",
                            issues,
                        );
                    }
                    validate_action(map, path, issues);
                }
                match kind {
                    "Input.ChoiceSet" => {
                        if let Some(choices) = map.get("choices") {
                            if let Some(arr) = choices.as_array() {
                                if arr.is_empty() {
                                    push_issue(
                                        path,
                                        "empty-choices",
                                        "Input.ChoiceSet must include at least one choice",
                                        issues,
                                    );
                                } else if arr.iter().any(|c| {
                                    !c.get("title")
                                        .and_then(|v| v.as_str())
                                        .map(|s| !s.is_empty())
                                        .unwrap_or(false)
                                        || !c
                                            .get("value")
                                            .and_then(|v| v.as_str())
                                            .map(|s| !s.is_empty())
                                            .unwrap_or(false)
                                }) {
                                    push_issue(
                                        path,
                                        "invalid-choice",
                                        "Choices must include non-empty title and value",
                                        issues,
                                    );
                                }
                            } else {
                                push_issue(
                                    path,
                                    "invalid-choices",
                                    "Input.ChoiceSet choices must be an array",
                                    issues,
                                );
                            }
                        } else {
                            push_issue(
                                path,
                                "missing-choices",
                                "Input.ChoiceSet must include choices",
                                issues,
                            );
                        }
                    }
                    "Input.Toggle" => {
                        if map
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .is_empty()
                        {
                            push_issue(
                                path,
                                "missing-title",
                                "Input.Toggle should include a title",
                                issues,
                            );
                        }
                    }
                    "Input.Number" => {
                        if let (Some(min), Some(max)) = (
                            map.get("min").and_then(|v| v.as_f64()),
                            map.get("max").and_then(|v| v.as_f64()),
                        ) && min > max
                        {
                            push_issue(
                                path,
                                "invalid-range",
                                "Input.Number min must be <= max",
                                issues,
                            );
                        }
                    }
                    "ColumnSet" => {
                        if let Some(columns) = map.get("columns") {
                            if !columns.is_array() {
                                push_issue(
                                    path,
                                    "invalid-columns",
                                    "ColumnSet columns must be an array",
                                    issues,
                                );
                            } else if columns.as_array().map(|c| c.is_empty()).unwrap_or(false) {
                                push_issue(
                                    path,
                                    "empty-columns",
                                    "ColumnSet columns must not be empty",
                                    issues,
                                );
                            }
                        }
                    }
                    "Media" => {
                        if let Some(sources) = map.get("sources") {
                            if !sources.is_array() {
                                push_issue(
                                    path,
                                    "invalid-sources",
                                    "Media sources must be an array",
                                    issues,
                                );
                            } else if sources.as_array().map(|s| s.is_empty()).unwrap_or(false) {
                                push_issue(
                                    path,
                                    "missing-sources",
                                    "Media must include at least one source",
                                    issues,
                                );
                            } else if sources
                                .as_array()
                                .map(|arr| {
                                    arr.iter().any(|s| {
                                        !s.get("url")
                                            .and_then(|v| v.as_str())
                                            .map(|v| !v.is_empty())
                                            .unwrap_or(false)
                                    })
                                })
                                .unwrap_or(false)
                            {
                                push_issue(
                                    path,
                                    "invalid-source",
                                    "Media sources must include non-empty url",
                                    issues,
                                );
                            }
                        } else {
                            push_issue(
                                path,
                                "missing-sources",
                                "Media must include sources",
                                issues,
                            );
                        }
                    }
                    _ => {}
                }
                for (key, value) in map {
                    let child_path = format!("{}/{}", path, key);
                    visit(value, &child_path, issues, input_ids, action_ids);
                }
            }
            Value::Array(items) => {
                for (idx, item) in items.iter().enumerate() {
                    let child_path = format!("{}/{}", path, idx);
                    visit(item, &child_path, issues, input_ids, action_ids);
                }
            }
            _ => {}
        }
    }

    fn validate_action(map: &Map<String, Value>, path: &str, issues: &mut Vec<ValidationIssue>) {
        let kind = map.get("type").and_then(|v| v.as_str()).unwrap_or_default();
        match kind {
            "Action.OpenUrl" => {
                if !map
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false)
                {
                    push_issue(
                        path,
                        "missing-url",
                        "Action.OpenUrl must include a url",
                        issues,
                    );
                }
            }
            "Action.Execute" => {
                if map.get("verb").and_then(|v| v.as_str()).is_none() {
                    push_issue(
                        path,
                        "missing-verb",
                        "Action.Execute should include a verb",
                        issues,
                    );
                }
                if map
                    .get("data")
                    .map(|d| !d.is_object() && !d.is_null())
                    .unwrap_or(false)
                {
                    push_issue(
                        path,
                        "invalid-data",
                        "Action.Execute data should be an object when present",
                        issues,
                    );
                }
            }
            "Action.ShowCard" => {
                if !map.contains_key("card") {
                    push_issue(
                        path,
                        "missing-card",
                        "Action.ShowCard must include a card",
                        issues,
                    );
                }
                if let Some(card_value) = map.get("card")
                    && !card_value.is_object()
                {
                    push_issue(
                        path,
                        "invalid-card",
                        "Action.ShowCard card must be an object",
                        issues,
                    );
                }
            }
            "Action.ToggleVisibility" => {
                if !map.contains_key("targetElements") {
                    push_issue(
                        path,
                        "missing-target-elements",
                        "Action.ToggleVisibility must include targetElements",
                        issues,
                    );
                } else if map
                    .get("targetElements")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.is_empty())
                    .unwrap_or(false)
                {
                    push_issue(
                        path,
                        "empty-target-elements",
                        "Action.ToggleVisibility targetElements must not be empty",
                        issues,
                    );
                }
            }
            _ => {}
        }
    }

    if let Some(body) = card.get("body")
        && !body.is_array()
    {
        push_issue(
            "/body",
            "invalid-body",
            "body must be an array",
            &mut issues,
        );
    }
    if let Some(actions) = card.get("actions")
        && !actions.is_array()
    {
        push_issue(
            "/actions",
            "invalid-actions",
            "actions must be an array",
            &mut issues,
        );
    }

    let mut action_ids = HashSet::new();
    visit(card, "", &mut issues, &mut input_ids, &mut action_ids);
    issues
}
