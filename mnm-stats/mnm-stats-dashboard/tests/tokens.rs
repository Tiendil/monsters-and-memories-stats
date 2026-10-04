//! Local format and rendering contracts; no source or network acquisition.
#[path = "../build_tokens.rs"]
mod adapter;

use serde_json::{Value, json};

fn token(kind: &str, value: Value) -> Value {
    json!({"$type":kind, "$value":value, "$description":"Fixture presentation role."})
}

fn generate(value: &Value) -> adapter::Generated {
    adapter::generate(&value.to_string(), "").unwrap_or_else(|e| panic!("{e}"))
}

fn rejected(value: &Value, path: &str, reason: &str) {
    let error = adapter::generate(&value.to_string(), "")
        .err()
        .expect("invalid tokens must fail");
    assert!(error.contains(path) && error.contains(reason), "{error}");
}

#[test]
fn structured_values_inheritance_aliases_and_deterministic_outputs() {
    let fixture = json!({
        "color": {"$type":"color", "ink":{"$value":{"colorSpace":"srgb","components":[0.2,0.4,0.6],"alpha":0.5},"$description":"Ink."}},
        "panel": token("color", json!("{color.ink}")),
        "chart": token("color", json!("{panel}")),
        "spacing": token("dimension", json!({"value":1.25,"unit":"rem"})),
        "font": token("fontFamily", json!(["Example Face", "sans-serif"])),
        "weight": token("fontWeight", json!(650)),
        "opacity": token("number", json!(0.5)),
        "shadow": token("shadow", json!({"color":"{panel}","offsetX":{"value":0,"unit":"px"},"offsetY":"{spacing}","blur":{"value":2,"unit":"rem"},"spread":{"value":-1,"unit":"px"},"inset":true})),
        "breakpoint": token("dimension", json!({"value":45,"unit":"rem"}))
    });
    let styles = "@media (max-width: token(breakpoint)) { p { color: var(--mnm-chart); } }";
    let a = adapter::generate(&fixture.to_string(), styles).unwrap();
    let b = adapter::generate(&serde_json::to_string_pretty(&fixture).unwrap(), styles).unwrap();
    assert_eq!(a.css, b.css);
    assert_eq!(a.rust, b.rust);
    for expected in [
        "--mnm-chart: rgba(51, 102, 153, 0.5)",
        "--mnm-spacing: 1.25rem",
        "--mnm-font: \"Example Face\", sans-serif",
        "--mnm-shadow: inset 0px 1.25rem 2rem -1px rgba(51, 102, 153, 0.5)",
        "@media (max-width: 45rem)",
    ] {
        assert!(a.css.contains(expected), "missing {expected}\n{}", a.css);
    }
    for expected in [
        r#"T_CHART: &str = "rgba(51, 102, 153, 0.5)""#,
        "Dimension::Rem(1.25)",
        "T_WEIGHT: f64 = 650.0",
        "inset: true",
    ] {
        assert!(a.rust.contains(expected), "missing {expected}\n{}", a.rust);
    }
    assert!(!a.css.contains("token(") && !a.css.contains("{panel}"));
}

#[test]
fn alias_and_output_name_failures_identify_the_token() {
    rejected(
        &json!({"surface":token("color",json!("{absent}"))}),
        "surface",
        "missing alias target absent",
    );
    rejected(
        &json!({"a":token("number",json!("{b}")), "b":token("number",json!("{a}"))}),
        "a",
        "cycle",
    );
    rejected(
        &json!({"a":token("number",json!(1)), "b":token("dimension",json!("{a}"))}),
        "b",
        "expected dimension",
    );
    rejected(
        &json!({"a-b":token("number",json!(1)), "a":{"b":token("number",json!(2))}}),
        "a.b",
        "CSS/Rust name collision",
    );
    rejected(
        &json!({"bad_name":token("number",json!(1))}),
        "bad_name",
        "unsupported",
    );
    assert!(
        adapter::generate(
            &json!({"a":token("number",json!(1))}).to_string(),
            "@media (max-width: token(missing)) {}"
        )
        .err()
        .unwrap()
        .contains("missing")
    );
    assert!(
        adapter::generate("{}", "p { color: var(--mnm-missing); }")
            .err()
            .unwrap()
            .contains("missing")
    );
}

#[test]
fn invalid_values_and_unsupported_features_fail_explicitly() {
    for (kind, value, reason) in [
        ("color", json!("#fff"), "structured"),
        (
            "color",
            json!({"colorSpace":"display-p3","components":[1,0,0]}),
            "sRGB",
        ),
        (
            "color",
            json!({"colorSpace":"srgb","components":[1,0,0],"alpha":2}),
            "0..=1",
        ),
        ("dimension", json!({"value":2,"unit":"em"}), "px or rem"),
        (
            "dimension",
            json!({"value":"2","unit":"px"}),
            "finite number",
        ),
        ("fontFamily", json!("Arial"), "structured"),
        ("fontFamily", json!([]), "nonempty"),
        ("fontWeight", json!(1001), "1..=1000"),
        ("number", json!({"$ref":"elsewhere"}), "finite number"),
        ("duration", json!({"value":1,"unit":"s"}), "unsupported"),
        ("shadow", json!([]), "empty"),
        (
            "shadow",
            json!({"color":{"colorSpace":"srgb","components":[0,0,0]}}),
            "missing offsetX",
        ),
    ] {
        rejected(&json!({"bad":token(kind,value)}), "bad", reason);
    }
    rejected(
        &json!({"group":{"$extends":"{another}"}}),
        "group",
        "unsupported",
    );
    rejected(
        &json!({"bad":{"$type":"number","$value":1}}),
        "bad",
        "$description",
    );
    rejected(
        &json!({"bad":{"$value":1,"$description":"missing type"}}),
        "bad",
        "$type",
    );
    rejected(
        &json!({"bad":{"$type":"number","$value":1,"$description":"Bad.","$extensions":{}}}),
        "bad",
        "unsupported",
    );
    rejected(
        &json!({"layer":{"tooltip":token("number",json!(1.5))}}),
        "layer.tooltip",
        "integer",
    );
    rejected(
        &json!({"opacity":{"disabled":token("number",json!(1.1))}}),
        "opacity.disabled",
        "0..=1",
    );
    rejected(
        &json!({"spacing":{"panel":token("dimension",json!({"value":-1,"unit":"rem"}))}}),
        "spacing.panel",
        "nonnegative",
    );
    for unit in ["rem", "px"] {
        let value = if unit == "rem" { 1.0 } else { 1.5 };
        rejected(
            &json!({"chart":{"viewport":{"min-height":token("dimension",json!({"value":value,"unit":unit}))}}}),
            "chart.viewport.min-height",
            "whole px",
        );
    }
}

#[test]
fn authored_tokens_generate_styles_and_typed_chart_values() {
    let source = include_str!("../../../specs/design-tokens.tokens.json");
    let styles = include_str!("../style.css");
    let result = adapter::generate(source, styles).unwrap();
    assert!(result.css.contains("@media (max-width: 1150px)"));
    assert!(result.css.contains("@media (max-width: 600px)"));
    assert!(!result.css.contains("token("));
    assert_eq!(
        mnm_stats_dashboard::charts::css_color(0),
        "rgba(139, 217, 198, 1)"
    );
    assert_eq!(
        mnm_stats_dashboard::tokens::T_SPACING_PANEL_PADDING,
        mnm_stats_dashboard::tokens::Dimension::Rem(1.5)
    );
    // Additional comparisons keep their own deterministic colors beyond the palette.
    assert_ne!(
        mnm_stats_dashboard::charts::css_color(6),
        mnm_stats_dashboard::charts::css_color(7)
    );
    let mut changed: Value = serde_json::from_str(source).unwrap();
    changed["chart"]["series"]["palette"]["01"]["$value"]["alpha"] = json!(0.25);
    let changed = generate(&changed);
    assert!(
        changed
            .css
            .contains("--mnm-chart-series-palette-01: rgba(139, 217, 198, 0.25)")
    );
    assert!(changed.rust.contains("rgba(139, 217, 198, 0.25)"));
}

#[test]
fn shared_scales_propagate_without_coupling_component_overrides() {
    let mut tokens: Value =
        serde_json::from_str(include_str!("../../../specs/design-tokens.tokens.json")).unwrap();
    tokens["scale"]["spacing"]["3"]["$value"]["value"] = json!(1);
    let shared = generate(&tokens);
    for role in [
        "input-padding",
        "table-cell-padding-block",
        "chip-padding-inline",
    ] {
        assert!(shared.css.contains(&format!("--mnm-spacing-{role}: 1rem;")));
    }
    tokens["spacing"]["input"]["padding"]["$value"] = json!("{scale.spacing.2}");
    let independent = generate(&tokens);
    assert!(
        independent
            .css
            .contains("--mnm-spacing-input-padding: 0.5rem;")
    );
    for role in ["table-cell-padding-block", "chip-padding-inline"] {
        assert!(
            independent
                .css
                .contains(&format!("--mnm-spacing-{role}: 1rem;"))
        );
    }
    // A heading change must not silently resize the correlation result.
    tokens["font"]["size"]["heading"]["section"]["$value"] = json!("{scale.font-size.5}");
    let independent = generate(&tokens);
    assert!(
        independent
            .css
            .contains("--mnm-font-size-heading-section: 1.25rem;")
    );
    assert!(
        independent
            .css
            .contains("--mnm-font-size-coefficient: 1.5rem;")
    );
}

#[test]
fn invalid_chart_consumer_types_and_geometry_report_semantic_paths() {
    let original: Value =
        serde_json::from_str(include_str!("../../../specs/design-tokens.tokens.json")).unwrap();
    let mut invalid = original.clone();
    invalid["chart"]["viewport"]["min-height"]["$value"]["value"] = json!(10);
    rejected(&invalid, "chart.viewport.min-height", "drawing space");
    let mut invalid = original.clone();
    invalid["chart"]["axis"]["label"]["font-size"]["$value"]["unit"] = json!("rem");
    rejected(&invalid, "chart.axis.label.font-size", "positive whole px");
    let mut invalid = original.clone();
    invalid["chart"]["axis"]["label"]["font-size"]["$value"]["value"] = json!(12.5);
    rejected(&invalid, "chart.axis.label.font-size", "positive whole px");
    let mut invalid = original;
    invalid["chart"]["axis"]["label"]
        .as_object_mut()
        .unwrap()
        .remove("font-family");
    rejected(
        &invalid,
        "chart.axis.label.font-family",
        "requires a fontFamily",
    );
}
