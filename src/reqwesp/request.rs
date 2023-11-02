use anyhow::{bail, Result};
use hyper::header::{HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::http::HeaderName;
use hyper::HeaderMap;
use serde::Serialize;

use embedded_svc::http::client::Client as HttpClient;
use embedded_svc::http::Method;
use esp_idf_svc::http::client::EspHttpConnection;

use crate::Response;

pub struct Request<'a> {
    pub(crate) method: Method,
    pub(crate) url: &'a str,
    pub(crate) headers: Box<[(&'a str, &'a str)]>,
    pub(crate) body: Option<Vec<u8>>,
}

pub struct RequestBuilder<'a> {
    pub(crate) client: &'a mut HttpClient<EspHttpConnection>,
    pub(crate) headers: HeaderMap,
    pub(crate) request: Result<Request<'a>>,
}

impl<'a> RequestBuilder<'a> {
    /// Constructs the `Request` and sends it to the target URL, returning a `Response`.
    pub fn send(&'a mut self) -> Result<Response<'a>> {
        match &mut self.request {
            Ok(ref mut request) => {
                // Convert from HeaderMap to a boxed slice used for the request
                request.headers = self
                    .headers
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.to_str().unwrap()))
                    .collect::<Vec<(&str, &str)>>()
                    .into_boxed_slice();

                Response::new(self.client, request)
            }
            Err(err) => bail!(err.to_string()),
        }
    }

    // TODO implement build method for returning a constructed `Request`.
    //  Must update `Request` headers with `RequestBuilder` headers before returning.
    // pub fn build(self) -> Result<Request<'a>> {
    //     self.request
    // }

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
            req.body = Some(data.to_vec());
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
                    req.body = Some(body);
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
