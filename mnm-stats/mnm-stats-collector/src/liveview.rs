//! The rendered-tree subset used by the captured metrics LiveView: static
//! fragments, nested dynamics, and comprehensions with shared templates.
//! Unsupported protocol forms fail rather than being silently omitted.

use crate::Result;
use serde_json::{Map, Value};

pub fn merge(current: &mut Value, diff: &Value) {
    if diff.get("s").is_some() || !current.is_object() || !diff.is_object() {
        *current = diff.clone();
        return;
    }
    for (key, value) in diff.as_object().unwrap() {
        let current = current
            .as_object_mut()
            .unwrap()
            .entry(key.clone())
            .or_insert(Value::Null);
        merge(current, value);
    }
}

pub fn render(value: &Value) -> Result<String> {
    render_node(value, &Map::new())
}

fn render_node(value: &Value, inherited: &Map<String, Value>) -> Result<String> {
    if let Some(text) = value.as_str() {
        return Ok(text.into());
    }
    let node = value
        .as_object()
        .ok_or("unsupported LiveView dynamic value")?;
    if node.keys().any(|key| {
        !matches!(key.as_str(), "s" | "p" | "d" | "r" | "t") && key.parse::<usize>().is_err()
    }) {
        return Err("unsupported LiveView rendering metadata".into());
    }
    let templates = match node.get("p") {
        Some(value) => value.as_object().ok_or("invalid LiveView template map")?,
        None => inherited,
    };
    let statics = node.get("s").ok_or("missing LiveView static fragments")?;
    let statics = if let Some(index) = statics.as_u64() {
        templates
            .get(&index.to_string())
            .ok_or("missing shared LiveView template")?
    } else {
        statics
    };
    let statics = statics
        .as_array()
        .ok_or("invalid LiveView static fragments")?;
    if statics.is_empty() {
        return Err("empty LiveView static template".into());
    }
    let render_row = |values: Vec<&Value>| -> Result<String> {
        if values.len() + 1 != statics.len() {
            return Err("LiveView template/dynamic length mismatch".into());
        }
        let mut result = String::new();
        for (i, part) in statics.iter().enumerate() {
            result.push_str(part.as_str().ok_or("non-string LiveView static fragment")?);
            if let Some(dynamic) = values.get(i) {
                result.push_str(&render_node(dynamic, templates)?);
            }
        }
        Ok(result)
    };
    if let Some(rows) = node.get("d") {
        let mut html = String::new();
        for row in rows.as_array().ok_or("invalid LiveView comprehension")? {
            html.push_str(&render_row(
                row.as_array()
                    .ok_or("invalid LiveView row")?
                    .iter()
                    .collect(),
            )?);
        }
        Ok(html)
    } else {
        let values = (0..statics.len() - 1)
            .map(|i| {
                node.get(&i.to_string())
                    .ok_or("missing LiveView dynamic fragment")
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        render_row(values)
    }
}

/// Render complete batches supplied by this update, excluding values merely
/// retained from the initial join. Server batches carry activity and charts together.
pub fn updated_batches(current: &Value, diff: &Value) -> Result<Vec<String>> {
    if diff.get("d").is_some() {
        return Ok(vec![render(current)?]);
    }
    let mut batches = Vec::new();
    if let Some(object) = diff.as_object() {
        for (key, value) in object {
            if key.parse::<usize>().is_ok() {
                batches.extend(updated_batches(&current[key], value)?);
            }
        }
    }
    Ok(batches)
}
