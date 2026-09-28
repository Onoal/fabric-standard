use std::collections::BTreeSet;

use crate::{Error, Method, Result};

#[derive(Clone)]
pub(crate) enum Segment {
    Static(String),
    Param(String),
    CatchAll(String),
}

#[derive(Clone)]
pub(crate) struct RoutePattern {
    original: String,
    segments: Vec<Segment>,
}

impl RoutePattern {
    pub(crate) fn parse(path: &str) -> Result<Self> {
        if !path.starts_with('/') {
            return Err(Error::authoring(format!(
                "route `{path}` must begin with /"
            )));
        }
        if path.contains('?') {
            return Err(Error::authoring(format!(
                "route `{path}` must not contain a query string"
            )));
        }
        let mut names = BTreeSet::new();
        let mut segments = Vec::new();
        let parts = path.split('/').skip(1).collect::<Vec<_>>();
        for (index, part) in parts.iter().enumerate() {
            if let Some(name) = part.strip_prefix(':') {
                validate_name(path, name)?;
                if !names.insert(name.to_owned()) {
                    return Err(Error::authoring(format!(
                        "route `{path}` contains duplicate parameter `{name}`"
                    )));
                }
                segments.push(Segment::Param(name.to_owned()));
            } else if let Some(name) = part.strip_prefix('*') {
                validate_name(path, name)?;
                if index != parts.len() - 1 {
                    return Err(Error::authoring(format!(
                        "catch-all in route `{path}` must be the final segment"
                    )));
                }
                if !names.insert(name.to_owned()) {
                    return Err(Error::authoring(format!(
                        "route `{path}` contains duplicate parameter `{name}`"
                    )));
                }
                segments.push(Segment::CatchAll(name.to_owned()));
            } else {
                segments.push(Segment::Static((*part).to_owned()));
            }
        }
        Ok(Self {
            original: path.to_owned(),
            segments,
        })
    }

    pub(crate) fn original(&self) -> &str {
        &self.original
    }

    pub(crate) fn prefixed(&self, prefix: &str) -> Result<Self> {
        let prefix = prefix.trim_end_matches('/');
        let path = if self.original == "/" {
            if prefix.is_empty() {
                "/".to_owned()
            } else {
                prefix.to_owned()
            }
        } else {
            format!("{prefix}{}", self.original)
        };
        Self::parse(&path)
    }

    pub(crate) fn captures(&self, path: &str) -> Result<Option<Vec<(String, String)>>> {
        let values = path.split('/').skip(1).collect::<Vec<_>>();
        let mut captured = Vec::new();
        let mut index = 0usize;
        for segment in &self.segments {
            match segment {
                Segment::Static(expected) => {
                    let Some(actual) = values.get(index) else {
                        return Ok(None);
                    };
                    if actual != expected {
                        return Ok(None);
                    }
                    index += 1;
                }
                Segment::Param(name) => {
                    let Some(actual) = values.get(index) else {
                        return Ok(None);
                    };
                    captured.push((name.clone(), percent_decode(actual)?));
                    index += 1;
                }
                Segment::CatchAll(name) => {
                    let rest = values[index..].join("/");
                    captured.push((name.clone(), percent_decode(&rest)?));
                    index = values.len();
                }
            }
        }
        if index == values.len() {
            Ok(Some(captured))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone)]
pub(crate) struct RouteKey {
    method: Method,
    pattern: String,
}

impl RouteKey {
    pub(crate) fn new(method: Method, pattern: &RoutePattern) -> Self {
        Self {
            method,
            pattern: pattern.original().to_owned(),
        }
    }

    pub(crate) fn matches(&self, method: &Method, pattern: &str) -> bool {
        &self.method == method && self.pattern == pattern
    }
}

fn validate_name(route: &str, name: &str) -> Result<()> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(Error::authoring(format!(
            "route `{route}` contains an empty parameter name"
        )));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(Error::authoring(format!(
            "route `{route}` has invalid parameter name `{name}`"
        )));
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(Error::authoring(format!(
            "route `{route}` has invalid parameter name `{name}`"
        )));
    }
    Ok(())
}

pub(crate) fn percent_decode(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(Error::dispatch(format!(
                    "invalid percent encoding in `{value}`"
                )));
            }
            let hi = hex(bytes[index + 1])
                .ok_or_else(|| Error::dispatch(format!("invalid percent encoding in `{value}`")))?;
            let lo = hex(bytes[index + 2])
                .ok_or_else(|| Error::dispatch(format!("invalid percent encoding in `{value}`")))?;
            output.push((hi << 4) | lo);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output)
        .map_err(|_| Error::dispatch(format!("percent decoded value is not UTF-8: `{value}`")))
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
