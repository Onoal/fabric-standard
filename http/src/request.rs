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
        Self {
            method,
            target,
            path,
            query,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let header = Header::new(name, value);
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
}

fn split_target(target: &str) -> (String, Option<String>) {
    match target.split_once('?') {
        Some((path, query)) => (path.to_owned(), Some(query.to_owned())),
        None => (target.to_owned(), None),
    }
}
