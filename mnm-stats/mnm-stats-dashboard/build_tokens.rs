//! Adapter for the project's authored DTCG profile, shared by the build and offline tests.
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

type Result<T> = std::result::Result<T, String>;

pub struct Generated {
    pub css: String,
    pub rust: String,
}

#[derive(Clone)]
struct Token {
    kind: String,
    value: Value,
}

fn error(path: &str, reason: &str) -> String {
    format!("{path}: {reason}")
}

fn object<'a>(value: &'a Value, path: &str, allowed: &[&str]) -> Result<&'a Map<String, Value>> {
    let fields = value
        .as_object()
        .ok_or_else(|| error(path, "expected an object"))?;
    if let Some(key) = fields.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(error(path, &format!("unsupported field {key}")));
    }
    Ok(fields)
}

fn collect(
    value: &Value,
    path: &str,
    inherited: Option<&str>,
    tokens: &mut BTreeMap<String, Token>,
) -> Result<()> {
    let fields = value
        .as_object()
        .ok_or_else(|| error(path, "expected a group or token"))?;
    let kind = match fields.get("$type") {
        Some(Value::String(kind)) => Some(kind.as_str()),
        Some(_) => return Err(error(path, "$type must be a string")),
        None => inherited,
    };
    if kind.is_some_and(|k| {
        !matches!(
            k,
            "color" | "dimension" | "fontFamily" | "fontWeight" | "number" | "shadow"
        )
    }) {
        return Err(error(path, "unsupported token type"));
    }
    if fields.get("$description").is_some_and(|v| !v.is_string()) {
        return Err(error(path, "$description must be a string"));
    }
    if fields.contains_key("$value") {
        if path.is_empty() {
            return Err(error("tokens", "tokens require a name"));
        }
        object(value, path, &["$type", "$value", "$description"])?;
        if !fields
            .get("$description")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
        {
            return Err(error(path, "token requires a nonempty $description"));
        }
        let kind = kind.ok_or_else(|| error(path, "missing explicit or inherited $type"))?;
        tokens.insert(
            path.into(),
            Token {
                kind: kind.into(),
                value: fields["$value"].clone(),
            },
        );
    } else {
        for (name, child) in fields {
            if name == "$type" || name == "$description" {
                continue;
            }
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err(error(
                    path,
                    &format!("unsupported group property or name {name}"),
                ));
            }
            let child_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            collect(child, &child_path, kind, tokens)?;
        }
    }
    Ok(())
}

fn number(value: &Value, path: &str) -> Result<f64> {
    value
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| error(path, "expected a finite number"))
}

fn bounded(value: &Value, path: &str, min: f64, max: f64) -> Result<f64> {
    let n = number(value, path)?;
    if !(min..=max).contains(&n) {
        return Err(error(path, &format!("number must be in {min}..={max}")));
    }
    Ok(n)
}

fn resolve(
    value: &Value,
    kind: &str,
    path: &str,
    tokens: &BTreeMap<String, Token>,
    visiting: &mut BTreeSet<String>,
) -> Result<Value> {
    if let Some(reference) = value.as_str() {
        let target = reference
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .ok_or_else(|| {
                error(
                    path,
                    "expected a structured value or complete {token.path} alias",
                )
            })?;
        let token = tokens
            .get(target)
            .ok_or_else(|| error(path, &format!("missing alias target {target}")))?;
        if token.kind != kind {
            return Err(error(
                path,
                &format!("alias {target} has type {}, expected {kind}", token.kind),
            ));
        }
        if !visiting.insert(target.into()) {
            return Err(error(path, &format!("alias cycle through {target}")));
        }
        let result =
            resolve(&token.value, kind, target, tokens, visiting).map_err(|e| error(path, &e));
        visiting.remove(target);
        return result;
    }
    match kind {
        "color" => {
            let fields = object(value, path, &["colorSpace", "components", "alpha"])?;
            if fields.get("colorSpace").and_then(Value::as_str) != Some("srgb") {
                return Err(error(path, "only structured sRGB colors are supported"));
            }
            let components = fields
                .get("components")
                .and_then(Value::as_array)
                .filter(|c| c.len() == 3)
                .ok_or_else(|| error(path, "color requires three numeric components"))?;
            for component in components {
                bounded(component, path, 0.0, 1.0)?;
            }
            if let Some(alpha) = fields.get("alpha") {
                bounded(alpha, path, 0.0, 1.0)?;
            }
        }
        "dimension" => {
            let fields = object(value, path, &["value", "unit"])?;
            number(&value["value"], path)?;
            if !matches!(
                fields.get("unit").and_then(Value::as_str),
                Some("px" | "rem")
            ) {
                return Err(error(path, "dimension unit must be px or rem"));
            }
        }
        "fontFamily" => {
            let families = value
                .as_array()
                .filter(|a| !a.is_empty())
                .ok_or_else(|| error(path, "fontFamily requires a nonempty array"))?;
            for family in families {
                if !family
                    .as_str()
                    .is_some_and(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
                {
                    return Err(error(
                        path,
                        "font family names must be nonempty strings without control characters",
                    ));
                }
            }
        }
        "fontWeight" => {
            bounded(value, path, 1.0, 1000.0)?;
        }
        "number" => {
            number(value, path)?;
        }
        "shadow" => {
            let shadows = value
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![value.clone()]);
            if shadows.is_empty() {
                return Err(error(path, "shadow array must not be empty"));
            }
            let mut resolved = Vec::new();
            for shadow in shadows {
                if shadow.is_string() {
                    let alias = resolve(&shadow, kind, path, tokens, visiting)?;
                    resolved.extend(alias.as_array().unwrap().iter().cloned());
                    continue;
                }
                let mut fields = object(
                    &shadow,
                    path,
                    &["color", "offsetX", "offsetY", "blur", "spread", "inset"],
                )?
                .clone();
                for (field, field_kind) in [
                    ("color", "color"),
                    ("offsetX", "dimension"),
                    ("offsetY", "dimension"),
                    ("blur", "dimension"),
                    ("spread", "dimension"),
                ] {
                    let raw = fields
                        .get(field)
                        .ok_or_else(|| error(path, &format!("shadow missing {field}")))?;
                    let result = resolve(raw, field_kind, path, tokens, visiting)?;
                    if field == "blur" && number(&result["value"], path)? < 0.0 {
                        return Err(error(path, "shadow blur must be nonnegative"));
                    }
                    fields.insert(field.into(), result);
                }
                if fields.get("inset").is_some_and(|v| !v.is_boolean()) {
                    return Err(error(path, "shadow inset must be boolean"));
                }
                resolved.push(Value::Object(fields));
            }
            return Ok(Value::Array(resolved));
        }
        _ => return Err(error(path, &format!("unsupported type {kind}"))),
    }
    Ok(value.clone())
}

fn rgba(value: &Value) -> (u8, u8, u8, f64) {
    // Emit identical rounded sRGB channels for CSS and Plotly configuration.
    let component = |i| (value["components"][i].as_f64().unwrap() * 255.0).round() as u8;
    (
        component(0),
        component(1),
        component(2),
        value["alpha"].as_f64().unwrap_or(1.0),
    )
}

fn css_value(kind: &str, value: &Value) -> String {
    match kind {
        "color" => {
            let (r, g, b, a) = rgba(value);
            format!("rgba({r}, {g}, {b}, {a})")
        }
        "dimension" => format!(
            "{}{}",
            value["value"].as_f64().unwrap(),
            value["unit"].as_str().unwrap()
        ),
        "fontFamily" => value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let s = v.as_str().unwrap();
                if matches!(
                    s,
                    "serif" | "sans-serif" | "monospace" | "system-ui" | "cursive" | "fantasy"
                ) {
                    s.into()
                } else {
                    format!(
                        "\"{}\"",
                        s.replace('\\', "\\\\")
                            .replace('"', "\\\"")
                            .replace('<', "\\3c ")
                    )
                }
            })
            .collect::<Vec<String>>()
            .join(", "),
        "shadow" => value
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                format!(
                    "{}{} {} {} {} {}",
                    if s["inset"] == true { "inset " } else { "" },
                    css_value("dimension", &s["offsetX"]),
                    css_value("dimension", &s["offsetY"]),
                    css_value("dimension", &s["blur"]),
                    css_value("dimension", &s["spread"]),
                    css_value("color", &s["color"])
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => value.as_f64().unwrap().to_string(),
    }
}

fn rust_value(kind: &str, value: &Value) -> (&'static str, String) {
    match kind {
        "color" => ("&str", format!("{:?}", css_value("color", value))),
        "dimension" => ("Dimension", format!("Dimension::{}({:?})", if value["unit"] == "px" { "Px" } else { "Rem" }, value["value"].as_f64().unwrap())),
        "fontFamily" => ("&[&str]", format!("&{:?}", value.as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect::<Vec<_>>())),
        "shadow" => ("&[Shadow]", format!("&[{}]", value.as_array().unwrap().iter().map(|s| format!("Shadow {{ color: {}, offset_x: {}, offset_y: {}, blur: {}, spread: {}, inset: {} }}",
            rust_value("color", &s["color"]).1, rust_value("dimension", &s["offsetX"]).1,
            rust_value("dimension", &s["offsetY"]).1, rust_value("dimension", &s["blur"]).1,
            rust_value("dimension", &s["spread"]).1, s["inset"].as_bool().unwrap_or(false)
        )).collect::<Vec<_>>().join(","))),
        _ => ("f64", format!("{:?}", value.as_f64().unwrap())),
    }
}

pub fn generate(source: &str, styles: &str) -> Result<Generated> {
    let document: Value =
        serde_json::from_str(source).map_err(|e| error("tokens", &e.to_string()))?;
    let mut tokens = BTreeMap::new();
    collect(&document, "", None, &mut tokens)?;
    let mut names = BTreeMap::new();
    let mut resolved = BTreeMap::new();
    for (path, token) in &tokens {
        let name = path.replace('.', "-");
        if let Some(other) = names.insert(name, path) {
            return Err(error(
                path,
                &format!("CSS/Rust name collision with {other}"),
            ));
        }
        resolved.insert(
            path.clone(),
            resolve(
                &token.value,
                &token.kind,
                path,
                &tokens,
                &mut BTreeSet::from([path.clone()]),
            )?,
        );
    }
    let mut css = String::from(":root {\n");
    let mut rust = String::from(
        "// Generated from design tokens. Do not edit.\n#[derive(Clone, Copy, Debug, PartialEq)]\npub enum Dimension { Px(f64), Rem(f64) }\nimpl Dimension { pub const fn pixels(self) -> u32 { self.px() as u32 } pub const fn px(self) -> f64 { match self { Self::Px(value) => value, Self::Rem(_) => panic!(\"relative dimension needs a CSS context\") } } }\n#[derive(Clone, Copy, Debug, PartialEq)]\npub struct Shadow { pub color: &'static str, pub offset_x: Dimension, pub offset_y: Dimension, pub blur: Dimension, pub spread: Dimension, pub inset: bool }\n",
    );
    let mut palette = Vec::new();
    for (path, value) in &resolved {
        let kind = &tokens[path].kind;
        if path.starts_with("opacity.") {
            bounded(value, path, 0.0, 1.0)?;
        }
        if path.starts_with("layer.") {
            let layer = bounded(value, path, i32::MIN as f64, i32::MAX as f64)?;
            if layer.fract() != 0.0 {
                return Err(error(path, "CSS stacking layer must be an integer"));
            }
        }
        if [
            "scale.spacing.",
            "spacing.",
            "radius.",
            "border.width.",
            "outline.focus.",
        ]
        .iter()
        .any(|prefix| path.starts_with(prefix))
            && (kind != "dimension" || value["value"].as_f64().is_none_or(|v| v < 0.0))
        {
            return Err(error(
                path,
                "CSS spacing, radius, border width, and focus geometry require a nonnegative dimension",
            ));
        }

        // Plotly dimensions use CSS pixels. Never interpret rem as a fixed pixel value.
        if matches!(
            path.as_str(),
            "chart.viewport.min-height"
                | "chart.axis.label.font-size"
                | "chart.viewport.margin"
                | "chart.axis.x.label-area"
                | "chart.axis.y.label-area"
                | "chart.series.line.width"
                | "chart.series.point.radius"
                | "chart.hover.hit-radius"
                | "chart.hover.series-min-height"
        ) && (kind != "dimension"
            || value["unit"] != "px"
            || value["value"]
                .as_f64()
                .is_none_or(|v| v <= 0.0 || v.fract() != 0.0 || v > i32::MAX as f64))
        {
            return Err(error(
                path,
                "chart geometry requires positive whole px values",
            ));
        }
        if matches!(
            path.as_str(),
            "chart.axis.x.range-padding"
                | "chart.series.fallback.saturation"
                | "chart.series.fallback.lightness"
                | "chart.series.fallback.hue-step"
                | "chart.series.fallback.alpha"
        ) {
            bounded(value, path, 0.0, 1.0)?;
        }
        let css_name = path.replace('.', "-");
        let rust_name = format!("T_{}", css_name.replace('-', "_").to_ascii_uppercase());
        writeln!(css, "  --mnm-{css_name}: {};", css_value(kind, value)).unwrap();
        let (ty, literal) = rust_value(kind, value);
        writeln!(rust, "pub const {rust_name}: {ty} = {literal};").unwrap();
        if kind == "fontFamily" {
            writeln!(
                rust,
                "pub const {rust_name}_CSS: &str = {:?};",
                css_value(kind, value)
            )
            .unwrap();
        }
        if path.starts_with("chart.series.palette.") {
            if kind != "color" {
                return Err(error(path, "chart palette entries must be colors"));
            }
            palette.push(rust_name);
        }
    }
    if tokens.keys().any(|p| p.starts_with("chart.")) {
        for (names, expected) in [
            (
                &[
                    "viewport.min-height",
                    "viewport.margin",
                    "axis.x.label-area",
                    "axis.y.label-area",
                    "axis.label.font-size",
                    "series.line.width",
                    "series.point.radius",
                    "hover.hit-radius",
                    "hover.series-min-height",
                ][..],
                "dimension",
            ),
            (
                &["axis.label.color", "axis.line.color", "grid.line.color"][..],
                "color",
            ),
            (
                &[
                    "axis.x.range-padding",
                    "series.fallback.hue-step",
                    "series.fallback.saturation",
                    "series.fallback.lightness",
                    "series.fallback.alpha",
                ][..],
                "number",
            ),
            (&["axis.label.font-family"][..], "fontFamily"),
        ] {
            for name in names {
                let path = format!("chart.{name}");
                if tokens.get(&path).is_none_or(|t| t.kind != expected) {
                    return Err(error(
                        &path,
                        &format!("chart consumer requires a {expected} token"),
                    ));
                }
            }
        }
        if palette.is_empty() {
            return Err(error("chart.series.palette", "requires at least one color"));
        }
        let pixels = |name: &str| {
            resolved[&format!("chart.{name}")]["value"]
                .as_f64()
                .unwrap()
        };
        if pixels("viewport.min-height")
            <= pixels("viewport.margin") * 2.0 + pixels("axis.x.label-area")
        {
            return Err(error(
                "chart.viewport.min-height",
                "must leave drawing space after margins and label area",
            ));
        }
    }
    if !palette.is_empty() {
        writeln!(
            rust,
            "pub const CHART_PALETTE: &[&str] = &[{}];",
            palette.join(",")
        )
        .unwrap();
    }
    css.push_str("}\n");
    let mut styles = styles.to_owned();
    while let Some(start) = styles.find("token(") {
        let end = styles[start..]
            .find(')')
            .map(|n| start + n)
            .ok_or_else(|| error("CSS", "unterminated token()"))?;
        let path = styles[start + 6..end].trim();
        let value = resolved
            .get(path)
            .ok_or_else(|| error(path, "unknown CSS token() reference"))?;
        if tokens[path].kind != "dimension" || value["value"].as_f64().unwrap() <= 0.0 {
            return Err(error(
                path,
                "CSS token() breakpoint requires a positive dimension",
            ));
        }
        styles.replace_range(start..=end, &css_value(&tokens[path].kind, value));
    }
    for reference in styles.split("var(--mnm-").skip(1) {
        let name = reference.split([')', ',']).next().unwrap().trim();
        if !names.contains_key(name) {
            return Err(error(name, "unknown CSS custom property token"));
        }
    }
    css.push_str(&styles);
    Ok(Generated { css, rust })
}
