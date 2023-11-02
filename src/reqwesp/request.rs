use anyhow::{bail, Result};
use hyper::body::Bytes;
use hyper::header::{HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::http::HeaderName;
use hyper::HeaderMap;
use serde::Serialize;

use embedded_svc::http::Method;

use crate::{Client, Response};

#[derive(Clone)]
pub struct Request<'a> {
    pub(crate) method: Method,
    pub(crate) url: &'a str,
    pub(crate) headers: Vec<(&'a str, &'a str)>,
    pub(crate) body: Option<Bytes>,
}

impl<'a> Request<'a> {
    pub fn new(method: Method, url: &'a str) -> Self {
        Self {
            method,
            url,
            headers: vec![],
            body: None,
        }
    }

    /// Get the method.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get a mutable reference to the method.
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    /// Get the url.
    pub fn url(&self) -> &str {
        self.url
    }

    /// Set a new url.
    pub fn set_url(&mut self, url: &'a str) {
        self.url = url
    }

    /// Get the headers.
    pub fn headers(&self) -> &Vec<(&'a str, &'a str)> {
        &self.headers
    }

    /// Get a mutable reference to the headers.
    pub fn headers_mut(&mut self) -> &mut Vec<(&'a str, &'a str)> {
        &mut self.headers
    }

    /// Get the body.
    pub fn body(&self) -> Option<&Bytes> {
        self.body.as_ref()
    }

    /// Get a mutable reference to the body.
    pub fn body_mut(&mut self) -> &mut Option<Bytes> {
        &mut self.body
    }
}

pub struct RequestBuilder<'a> {
    client: &'a mut Client,
    headers: HeaderMap,
    request: Result<Request<'a>>,
}

impl<'a> RequestBuilder<'a> {
    pub fn new(client: &'a mut Client, method: Method, url: &'a str) -> Self {
        Self {
            client,
            headers: HeaderMap::new(),
            request: Ok(Request::new(method, url)),
        }
    }

    /// Constructs the `Request` and sends it to the target URL, returning a `Response`.
    pub fn send(&'a mut self) -> Result<Response> {
        match &mut self.request {
            Ok(ref mut request) => {
                // Convert from HeaderMap to vec
                request.headers = self
                    .headers
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.to_str().unwrap()))
                    .collect();

                self.client.execute(request)
            }
            Err(err) => bail!(err.to_string()),
        }
    }

    /// Build a `Request`, which can be inspected, modified and executed with
    /// `Client::execute()`.
    pub fn build(&'a mut self) -> Result<Request> {
        match &mut self.request {
            Ok(ref mut request) => {
                // Convert from HeaderMap to vec
                request.headers = self
                    .headers
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.to_str().unwrap()))
                    .collect();

                Ok(request.clone())
            }
            Err(err) => bail!(err.to_string()),
        }
    }

    /// Add a header to this request.
    pub fn header(mut self, key: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Add multiple headers to this request.
    pub fn headers(mut self, headers: Vec<(HeaderName, HeaderValue)>) -> Self {
        for (key, value) in headers.into_iter() {
            self.headers.insert(key, value);
        }
        self
    }

    /// Add a body to this request.
    pub fn body(mut self, data: &[u8]) -> Self {
        let content_length_header = data.len().to_string();
        if let Ok(ref mut req) = self.request {
            req.body = Some(Bytes::copy_from_slice(data));
        }
        self.header(CONTENT_LENGTH, content_length_header.parse().unwrap())
    }

    // TODO Add method to modify the query string of the URL.
    //  Consider finding an external crate for parsing and handling URLs.
    // pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> Self {}

    /// Send a form body.
    pub fn form<T: Serialize + ?Sized>(mut self, form: &'a T) -> Self {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match serde_urlencoded::to_string(form) {
                Ok(body) => {
                    let content_length_header = body.len().to_string();
                    self.headers
                        .insert(CONTENT_LENGTH, content_length_header.parse().unwrap());
                    self.headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                    req.body = Some(body.into());
                }
                Err(err) => error = Some(crate::error::builder(err)),
            }
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Add a json body to this request.
    pub fn json(mut self, json: &impl Serialize) -> Self {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match serde_json::to_vec(json) {
                Ok(body) => {
                    let content_length_header = body.len().to_string();
                    self.headers
                        .insert(CONTENT_LENGTH, content_length_header.parse().unwrap());
                    if !self.headers.contains_key(CONTENT_TYPE) {
                        self.headers
                            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                    }
                    req.body = Some(Bytes::from(body));
                }
                Err(err) => error = Some(err),
            }
        }
        if let Some(err) = error {
            self.request = Err(anyhow::Error::from(err));
        }
        self
    }
}
