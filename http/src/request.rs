use fabric_package_networking_http::{HttpHeader, HttpRequest, HttpVersion};

use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Head,
    Other(String),
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Options => "OPTIONS",
            Self::Head => "HEAD",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl From<&str> for Method {
    fn from(value: &str) -> Self {
        match value {
            "GET" => Self::Get,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "PATCH" => Self::Patch,
            "DELETE" => Self::Delete,
            "OPTIONS" => Self::Options,
            "HEAD" => Self::Head,
            other => Self::Other(other.to_owned()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl Header {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    method: Method,
    target: String,
    path: String,
    query: Option<String>,
    headers: Vec<Header>,
    body: Vec<u8>,
    raw: HttpRequest,
}

impl Request {
    pub fn get(target: impl Into<String>) -> Self {
        Self::new(Method::Get, target, Vec::new())
    }

    pub fn post(target: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self::new(Method::Post, target, body)
    }

    pub fn new(method: Method, target: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        let target = target.into();
        let (path, query) = split_target(&target);
        let body = body.into();
        let raw = HttpRequest {
            method: method.as_str().to_owned(),
            target: target.clone(),
            version: HttpVersion::Http11,
            headers: Vec::new(),
            body: body.clone(),
        };
        Self {
            method,
            target,
            path,
            query,
            headers: Vec::new(),
            body,
            raw,
        }
    }

    pub fn from_http(raw: HttpRequest) -> Result<Self> {
        let method = Method::from(raw.method.as_str());
        let (path, query) = split_target(&raw.target);
        if !path.starts_with('/') {
            return Err(Error::dispatch(format!(
                "HTTP request target `{}` does not contain an absolute path",
                raw.target
            )));
        }
        let headers = raw
            .headers
            .iter()
            .map(|header| Header::new(header.name.clone(), header.value.clone()))
            .collect();
        Ok(Self {
            method,
            target: raw.target.clone(),
            path,
            query,
            headers,
            body: raw.body.clone(),
            raw,
        })
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let header = Header::new(name, value);
        self.raw
            .headers
            .push(HttpHeader::new(header.name.clone(), header.value.clone()));
        self.headers.push(header);
        self
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    pub fn query_values(&self, key: &str) -> Vec<&str> {
        self.query
            .as_deref()
            .into_iter()
            .flat_map(|query| query.split('&'))
            .filter_map(|pair| {
                let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
                (name == key).then_some(value)
            })
            .collect()
    }

    pub fn query_first(&self, key: &str) -> Option<&str> {
        self.query_values(key).into_iter().next()
    }

    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }

    pub fn headers_all(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
            .collect()
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn raw(&self) -> &HttpRequest {
        &self.raw
    }
}

impl From<Header> for HttpHeader {
    fn from(value: Header) -> Self {
        Self::new(value.name, value.value)
    }
}

fn split_target(target: &str) -> (String, Option<String>) {
    match target.split_once('?') {
        Some((path, query)) => (path.to_owned(), Some(query.to_owned())),
        None => (target.to_owned(), None),
    }
}
