use anyhow::Result;
use encoding_rs::{Encoding, UTF_8};
use hyper::body::Bytes;
use hyper::header::HeaderValue;
use hyper::StatusCode;
use mime::Mime;
use serde::de::DeserializeOwned;

use embedded_svc::http::client::{Client as HttpClient, Response as HttpResponse};
use embedded_svc::io::Write;
use esp_idf_svc::http::client::EspHttpConnection;

use crate::reqwesp::Request;

pub struct Response<'a> {
    body: Bytes,
    res: HttpResponse<&'a mut EspHttpConnection>,
    url: &'a str,
}

impl<'a> Response<'a> {
    pub(crate) fn new(
        client: &'a mut HttpClient<EspHttpConnection>,
        request: &'a Request<'a>,
    ) -> Result<Self> {
        let mut req = client.request(request.method, request.url, request.headers.as_slice())?;

        if let Some(data) = &request.body {
            log::debug!("Adding data to request: {} bytes", data.len());
            req.write_all(data)?;
            req.flush()?;
        }

        let mut response = Self {
            body: Bytes::new(),
            res: req.submit()?,
            url: request.url,
        };
        response.read()?;
        Ok(response)
    }

    // Read the `HttpResponse` into the `Response` body as `Bytes`.
    fn read(&mut self) -> Result<()> {
        // Use a vector so we don't need to know the max size of the response
        let mut data = Vec::new();
        let mut buf = [0u8; 256];
        // Read into buffer and append to vector until the reader is empty
        loop {
            let size = self.res.read(&mut buf)?;
            if size == 0 {
                break;
            }
            data.extend_from_slice(&buf[..size])
        }

        self.body = Bytes::from(data);
        Ok(())
    }

    /// Get the `StatusCode` of this `Response`.
    pub fn status(&self) -> StatusCode {
        match StatusCode::from_u16(self.res.status()) {
            Ok(status) => status,
            Err(_) => {
                log::error!("Invalid response status code");
                StatusCode::BAD_REQUEST
            }
        }
    }

    /// Get the final URL of this `Response`.
    pub fn url(&self) -> &str {
        self.url
    }

    /// Obtain the given header.
    pub fn header(&self, name: &str) -> Option<HeaderValue> {
        let raw_value = self.res.header(name)?;
        match HeaderValue::from_str(raw_value) {
            Ok(header) => Some(header),
            Err(_) => {
                log::error!(
                    "Header value for exists, but is invalid: name=`{name}`, value=`{raw_value}`"
                );
                None
            }
        }
    }

    /// Get the full response text.
    pub fn text(self) -> Result<String> {
        self.text_with_charset("utf-8")
    }

    /// Get the full response text given a specific encoding.
    pub fn text_with_charset(self, default_encoding: &str) -> Result<String> {
        let content_type = self
            .header("Content-Type")
            .as_ref()
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());

        let encoding_name = content_type
            .as_ref()
            .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
            .unwrap_or(default_encoding);
        let encoding = Encoding::for_label(encoding_name.as_bytes()).unwrap_or(UTF_8);

        let full = self.bytes();

        let (text, _, _) = encoding.decode(&full);
        Ok(text.into_owned())
    }

    /// Try to deserialize the response body as JSON.
    pub fn json<T: DeserializeOwned>(self) -> Result<T> {
        let full = self.bytes();
        serde_json::from_slice(&full).map_err(crate::error::decode)
    }

    /// Get the full response body as `Bytes`.
    pub fn bytes(self) -> Bytes {
        self.body
    }

    /// Turn a response into an error if the server returned an error.
    pub fn error_for_status(self) -> Result<Self> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            Err(crate::error::status_code(self.url, status))
        } else {
            Ok(self)
        }
    }

    /// Turn a reference to a response into an error if the server returned an error.
    pub fn error_for_status_ref(&self) -> Result<&Self> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            Err(crate::error::status_code(self.url, status))
        } else {
            Ok(self)
        }
    }
}
