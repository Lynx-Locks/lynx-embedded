use anyhow::Result;

use embedded_svc::http::client::Client as HttpClient;
use embedded_svc::http::Method;
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};

use crate::{Request, RequestBuilder, Response};

pub struct Client {
    client: HttpClient<EspHttpConnection>,
}

impl<'a> Client {
    /// Constructs a new `Client`.
    pub fn new() -> Result<Self> {
        Ok(Client {
            client: Self::create_http_client()?,
        })
    }

    /// Start building a `Request` with the `Method` and url.
    pub fn request(&'a mut self, method: Method, url: &'a str) -> RequestBuilder {
        RequestBuilder::new(self, method, url)
    }

    /// Convenience method to make a `GET` request to a URL.
    pub fn get(&'a mut self, url: &'a str) -> RequestBuilder {
        self.request(Method::Get, url)
    }

    /// Convenience method to make a `POST` request to a URL.
    pub fn post(&'a mut self, url: &'a str) -> RequestBuilder {
        self.request(Method::Post, url)
    }

    /// Convenience method to make a `PUT` request to a URL.
    pub fn put(&'a mut self, url: &'a str) -> RequestBuilder {
        self.request(Method::Put, url)
    }

    /// Convenience method to make a `DELETE` request to a URL.
    pub fn delete(&'a mut self, url: &'a str) -> RequestBuilder {
        self.request(Method::Delete, url)
    }

    /// Execute a `Request`.
    ///
    /// A `Request` can be built manually with `Request::new()` or obtained
    /// from a RequestBuilder with `RequestBuilder::build()`.
    ///
    /// You should prefer to use the `RequestBuilder` and
    /// `RequestBuilder::send()`.
    pub fn execute(&'a mut self, request: &'a Request) -> Result<Response> {
        Response::new(&mut self.client, request)
    }

    /// Create a new `HttpClient` with a `EspHttpConnection` handler.
    fn create_http_client() -> Result<HttpClient<EspHttpConnection>> {
        // Create HTTPS Connection Handler
        let connection = EspHttpConnection::new(&HttpConfig {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        })?;

        // Create HTTPS Client
        let client = HttpClient::wrap(connection);
        Ok(client)
    }
}
