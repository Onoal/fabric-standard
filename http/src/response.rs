use serde::Serialize;

use crate::{Header, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    status: u16,
    headers: Vec<Header>,
    body: Vec<u8>,
}

impl Response {
    pub fn text(body: impl Into<String>) -> Self {
        Self::bytes(body.into().into_bytes())
            .append_header_static("content-type", "text/plain; charset=utf-8")
    }

    pub fn json(value: &impl Serialize) -> Result<Self> {
        let body = serde_json::to_vec(value)?;
        Ok(Self::bytes(body).append_header_static("content-type", "application/json"))
    }

    pub fn bytes(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    pub fn redirect(location: impl Into<String>) -> Self {
        Self::bytes(Vec::new())
            .with_status(302)
            .append_header_static("location", location)
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn set_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn append_header(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<()> {
        self.headers.push(Header::new(name, value));
        Ok(())
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(Header::new(name, value));
        self
    }

    pub(crate) fn strip_body(mut self) -> Self {
        self.body.clear();
        self
    }

    fn append_header_static(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(Header::new(name, value));
        self
    }
}
