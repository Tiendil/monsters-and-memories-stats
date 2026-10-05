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
    assert_eq!(a.resolved, b.resolved);
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
    let source = include_str!("../design-tokens.tokens.json");
    let styles = include_str!("../style.css");
    let result = adapter::generate(source, styles).unwrap();
    assert!(result.css.contains("@media (max-width: 1150px)"));
    assert!(result.css.contains("@media (max-width: 600px)"));
    assert!(!result.css.contains("token("));
    assert_eq!(
        mnm_stats_dashboard::charts::css_color(0),
        "rgba(0, 120, 111, 1)"
    );
    assert_eq!(
        mnm_stats_dashboard::tokens::T_SPACING_PANEL_PADDING,
        mnm_stats_dashboard::tokens::Dimension::Rem(1.0)
    );
    // Additional comparisons keep their own deterministic colors beyond the palette.
    assert_ne!(
        mnm_stats_dashboard::charts::css_color(6),
        mnm_stats_dashboard::charts::css_color(7)
    );
    let mut changed: Value = serde_json::from_str(source).unwrap();
    changed["chart"]["series"]["palette"]["01"]["$value"] = json!("{tailwind.color.white}");
    let changed = generate(&changed);
    assert!(
        changed
            .css
            .contains("--mnm-chart-series-palette-01: rgba(255, 255, 255, 1)")
    );
    assert!(changed.rust.contains("rgba(255, 255, 255, 1)"));
}

#[test]
fn shared_scales_propagate_without_coupling_component_overrides() {
    let mut tokens: Value =
        serde_json::from_str(include_str!("../design-tokens.tokens.json")).unwrap();
    tokens["scale"]["spacing"]["3"]["$value"] = json!("{tailwind.spacing.4}");
    let shared = generate(&tokens);
    for role in [
        "input-padding",
        "summary-padding-block",
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
    for role in ["summary-padding-block", "chip-padding-inline"] {
        assert!(
            independent
                .css
                .contains(&format!("--mnm-spacing-{role}: 1rem;"))
        );
    }
    // A heading change must not silently resize compact headline counts.
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
            .contains("--mnm-font-size-summary-compact: 1.5rem;")
    );
}

#[test]
fn invalid_chart_consumer_types_and_geometry_report_semantic_paths() {
    let original: Value =
        serde_json::from_str(include_str!("../design-tokens.tokens.json")).unwrap();
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

#[test]
fn tailwind_imports_keep_theme_values_and_semantic_consumers_in_sync() {
    let fixture = json!({
        "ink":token("color",json!("{tailwind.color.example.700}")),
        "accent":token("color",json!("{ink}")),
        "space":token("dimension",json!("{tailwind.spacing.4}")),
        "size":token("dimension",json!("{tailwind.text.sm}")),
        "rounding":token("dimension",json!("{tailwind.radius.md}")),
        "font":token("fontFamily",json!("{tailwind.font.sans}")),
        "weight":token("fontWeight",json!("{tailwind.font-weight.medium}")),
        "leading":token("number",json!("{tailwind.leading.normal}"))
    });
    let theme = r#"
        :root { --color-example-700: red; }
        @theme default {
            --color-example-700: oklch(50% 0 0 / 0.5);
            --spacing: 0.25rem;
            --text-sm: 0.875rem;
            --radius-md: 0.375rem;
            --font-sans: 'Example Face',
                Arial, sans-serif;
            --font-weight-medium: 500;
            --leading-normal: 1.5;
            @keyframes pulse { to { --color-example-700: red; } }
        }
        @theme default inline reference { --unused: unsupported(1); }
        .other { --spacing: 10rem; }
    "#;
    let a = adapter::generate_with_theme(&fixture.to_string(), "", theme).unwrap();
    for expected in [
        "--mnm-accent: rgba(99, 99, 99, 0.5);",
        "--mnm-space: 1rem;",
        "--mnm-size: 0.875rem;",
        "--mnm-rounding: 0.375rem;",
        "--mnm-font: \"Example Face\", \"Arial\", sans-serif;",
        "--mnm-weight: 500;",
        "--mnm-leading: 1.5;",
    ] {
        assert!(a.css.contains(expected), "missing {expected} in {}", a.css);
    }
    assert!(a.rust.contains("rgba(99, 99, 99, 0.5)"));
    let resolved: Value = serde_json::from_str(&a.resolved).unwrap();
    assert_eq!(resolved["accent"]["$value"], resolved["ink"]["$value"]);
    assert!(!a.resolved.contains("{tailwind.") && !a.resolved.contains("{ink}"));
    let roundtrip = adapter::generate_with_theme(&a.resolved, "", "").unwrap();
    assert_eq!(roundtrip.css, a.css);
    assert_eq!(roundtrip.rust, a.rust);
    assert_eq!(roundtrip.resolved, a.resolved);
    let changed = theme
        .replace("oklch(50% 0 0 / 0.5)", "#fff")
        .replace("0.25rem", "0.5rem");
    let b = adapter::generate_with_theme(&fixture.to_string(), "", &changed).unwrap();
    assert!(b.css.contains("--mnm-accent: rgba(255, 255, 255, 1);"));
    assert!(b.rust.contains("rgba(255, 255, 255, 1)"));
    assert!(b.css.contains("--mnm-space: 2rem;"));
    assert!(b.rust.contains("T_SPACE: Dimension = Dimension::Rem(2.0)"));
}

#[test]
fn invalid_tailwind_imports_report_the_semantic_role() {
    for (kind, reference, reason) in [
        (
            "color",
            "tailwind.color.missing.700",
            "missing Tailwind reference",
        ),
        ("color", "tailwind.spacing.4", "expected color"),
        ("dimension", "tailwind.spacing.small", "spacing step"),
        (
            "number",
            "tailwind.unknown.value",
            "unsupported Tailwind family",
        ),
    ] {
        rejected(
            &json!({"accent":token(kind,json!(format!("{{{reference}}}")))}),
            "accent",
            reason,
        );
    }
    rejected(
        &json!({"tailwind":{"color":token("number",json!(1))}}),
        "tailwind",
        "reserved",
    );
    let fixture = json!({"accent":token("color",json!("{tailwind.color.invalid}"))});
    let error = adapter::generate_with_theme(
        &fixture.to_string(),
        "",
        "@theme { --color-invalid: var(--missing); }",
    )
    .err()
    .unwrap();
    assert!(
        error.contains("accent") && error.contains("unsupported Tailwind value"),
        "{error}"
    );
}
