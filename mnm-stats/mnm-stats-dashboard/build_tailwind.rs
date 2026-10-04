//! Import the pinned, upstream Tailwind theme installed by the development image.
use cssparser::{Delimiter, ParseError, Parser, ParserInput, Token};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const SOURCE: &str = include_str!("/opt/tailwindcss/theme.css");

pub struct Theme(BTreeMap<String, String>);

impl Theme {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut input = ParserInput::new(source);
        let mut parser = Parser::new(&mut input);
        let mut values = BTreeMap::new();
        while let Ok(token) = parser.next() {
            if !matches!(token, Token::AtKeyword(name) if *name == "theme") {
                continue;
            }
            // Skip @theme modifiers; cssparser skips other unopened nested blocks.
            while let Ok(token) = parser.next() {
                match token {
                    Token::Semicolon => break,
                    Token::CurlyBracketBlock => {
                        parser
                            .parse_nested_block(|block| {
                                while let Ok(token) = block.next() {
                                    let Token::Ident(name) = token else { continue };
                                    if !name.starts_with("--") {
                                        continue;
                                    }
                                    let name = name.to_string();
                                    block.expect_colon()?;
                                    let value =
                                        block.parse_until_after(Delimiter::Semicolon, |value| {
                                            let start = value.position();
                                            while value.next().is_ok() {}
                                            Ok::<_, ParseError<'_, ()>>(
                                                value.slice_from(start).trim().to_owned(),
                                            )
                                        })?;
                                    values.insert(name, value);
                                }
                                Ok::<_, ParseError<'_, ()>>(())
                            })
                            .map_err(|e| format!("Tailwind theme: {e:?}"))?;
                        break;
                    }
                    _ => {}
                }
            }
        }
        Ok(Self(values))
    }

    pub fn resolve(&self, reference: &str, expected: &str) -> Result<Value, String> {
        let name = reference.strip_prefix("tailwind.").unwrap();
        let (family, suffix) = name
            .split_once('.')
            .ok_or_else(|| format!("invalid Tailwind reference {reference}"))?;
        let kind = match family {
            "color" => "color",
            "spacing" | "text" | "radius" => "dimension",
            "font" => "fontFamily",
            "font-weight" => "fontWeight",
            "leading" => "number",
            _ => return Err(format!("unsupported Tailwind family {family}")),
        };
        if kind != expected {
            return Err(format!(
                "alias {reference} has type {kind}, expected {expected}"
            ));
        }
        let variable = if family == "spacing" {
            "--spacing".to_owned()
        } else {
            format!("--{}", name.replace('.', "-"))
        };
        let css = self
            .0
            .get(&variable)
            .ok_or_else(|| format!("missing Tailwind reference {reference} ({variable})"))?;
        let invalid = || format!("unsupported Tailwind value for {reference}: {css}");
        if kind == "color" {
            let color = csscolorparser::parse(css).map_err(|_| invalid())?;
            let [r, g, b, a] = color.to_array();
            return Ok(json!({"colorSpace":"srgb", "components":[r,g,b], "alpha":a}));
        }
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let value = match kind {
            "dimension" => {
                let Token::Dimension { value, unit, .. } = parser.next().map_err(|_| invalid())?
                else {
                    return Err(invalid());
                };
                if !matches!(unit.as_ref(), "px" | "rem") {
                    return Err(invalid());
                }
                let multiplier = if family == "spacing" {
                    suffix.parse::<u32>().map_err(|_| {
                        format!("{reference}: spacing step must be a nonnegative integer")
                    })? as f64
                } else {
                    1.0
                };
                json!({"value": f64::from(*value) * multiplier, "unit": unit.as_ref()})
            }
            "fontFamily" => {
                let families = parser
                    .parse_comma_separated(|part| {
                        let mut words = vec![part.expect_ident_or_string()?.to_string()];
                        while !part.is_exhausted() {
                            words.push(part.expect_ident()?.to_string());
                        }
                        Ok::<_, ParseError<'_, ()>>(words.join(" "))
                    })
                    .map_err(|_| invalid())?;
                json!(families)
            }
            _ => json!(parser.expect_number().map_err(|_| invalid())?),
        };
        parser.expect_exhausted().map_err(|_| invalid())?;
        Ok(value)
    }
}
