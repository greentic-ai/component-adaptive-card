use std::path::Path;

/// Ensures the custom config flow template includes a terminal route so config-time nodes can be
/// inserted into empty flows without triggering ADD_STEP_ROUTING_MISSING.
#[test]
fn custom_flow_emits_terminal_route() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("flows/custom.ygtc");
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));

    assert!(
        contents.contains(r#""to": "NEXT_NODE_PLACEHOLDER""#),
        "custom config flow template must still include NEXT_NODE_PLACEHOLDER so add-step can thread routing; check {}",
        path.display()
    );
    assert!(
        contents.contains(r#""out": true"#),
        "custom config flow template must emit an explicit terminal route so config-mode nodes aren't stuck; check {}",
        path.display()
    );
}
