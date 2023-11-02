use std::error::Error as StdError;
use std::fmt;

use hyper::StatusCode;

pub trait HttpError: StdError + Send + Sync + 'static {}
impl<T> HttpError for T where T: StdError + Send + Sync + 'static {}

pub struct Error<E: HttpError> {
    inner: Box<Inner<E>>,
}

struct Inner<E: HttpError> {
    kind: Kind,
    source: Option<E>,
    url: Option<String>,
}

impl<E: HttpError> Error<E> {
    pub(crate) fn new(kind: Kind, source: Option<E>) -> Error<E> {
        Error {
            inner: Box::new(Inner {
                kind,
                source: source.map(Into::into),
                url: None,
            }),
        }
    }

    pub fn url(&self) -> Option<&String> {
        self.inner.url.as_ref()
    }

    /// Returns a mutable reference to the URL related to this error
    ///
    /// This is useful if you need to remove sensitive information from the URL
    /// (e.g. an API key in the query), but do not want to remove the URL
    /// entirely.
    pub fn url_mut(&mut self) -> Option<&mut String> {
        self.inner.url.as_mut()
    }

    /// Add a url related to this error (overwriting any existing)
    pub fn with_url(mut self, url: String) -> Self {
        self.inner.url = Some(url);
        self
    }

    /// Strip the related url from this error (if, for example, it contains
    /// sensitive information)
    pub fn without_url(mut self) -> Self {
        self.inner.url = None;
        self
    }

    /// Returns true if the error is from a type Builder.
    pub fn is_builder(&self) -> bool {
        matches!(self.inner.kind, Kind::Builder)
    }

    /// Returns true if the error is from a `RedirectPolicy`.
    pub fn is_redirect(&self) -> bool {
        matches!(self.inner.kind, Kind::Redirect)
    }

    /// Returns true if the error is from `Response::error_for_status`.
    pub fn is_status(&self) -> bool {
        matches!(self.inner.kind, Kind::Status(_))
    }

    /// Returns true if the error is related to the request
    pub fn is_request(&self) -> bool {
        matches!(self.inner.kind, Kind::Request)
    }

    /// Returns true if the error is related to the request or response body
    pub fn is_body(&self) -> bool {
        matches!(self.inner.kind, Kind::Body)
    }

    /// Returns true if the error is related to decoding the response's body
    pub fn is_decode(&self) -> bool {
        matches!(self.inner.kind, Kind::Decode)
    }

    /// Returns the status code, if the error was generated from a response.
    pub fn status(&self) -> Option<StatusCode> {
        match self.inner.kind {
            Kind::Status(code) => Some(code),
            _ => None,
        }
    }
}

impl<E: HttpError> fmt::Debug for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("reqwesp::Error");

        builder.field("kind", &self.inner.kind);

        if let Some(ref url) = self.inner.url {
            builder.field("url", url);
        }
        if let Some(ref source) = self.inner.source {
            builder.field("source", source);
        }

        builder.finish()
    }
}

impl<E: HttpError> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.inner.kind {
            Kind::Builder => f.write_str("builder error")?,
            Kind::Request => f.write_str("error sending request")?,
            Kind::Body => f.write_str("request or response body error")?,
            Kind::Decode => f.write_str("error decoding response body")?,
            Kind::Redirect => f.write_str("error following redirect")?,
            Kind::Upgrade => f.write_str("error upgrading connection")?,
            Kind::Status(ref code) => {
                let prefix = if code.is_client_error() {
                    "HTTP status client error"
                } else {
                    debug_assert!(code.is_server_error());
                    "HTTP status server error"
                };
                write!(f, "{prefix} ({code})")?;
            }
        };

        if let Some(url) = &self.inner.url {
            write!(f, " for url ({})", url.as_str())?;
        }

        if let Some(e) = &self.inner.source {
            write!(f, ": {e}")?;
        }

        Ok(())
    }
}

impl<E: HttpError> StdError for Error<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source.as_ref().map(|e| e as _)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum Kind {
    Builder,
    Request,
    Redirect,
    Status(StatusCode),
    Body,
    Decode,
    Upgrade,
}

// constructors

pub(crate) fn builder(e: impl HttpError) -> anyhow::Error {
    anyhow::Error::new(Error::new(Kind::Builder, Some(e)))
}

#[allow(dead_code)]
pub(crate) fn body(e: impl HttpError) -> anyhow::Error {
    anyhow::Error::new(Error::new(Kind::Body, Some(e)))
}

pub(crate) fn decode(e: impl HttpError) -> anyhow::Error {
    anyhow::Error::new(Error::new(Kind::Decode, Some(e)))
}

#[allow(dead_code)]
pub(crate) fn request(e: impl HttpError) -> anyhow::Error {
    anyhow::Error::new(Error::new(Kind::Request, Some(e)))
}

#[allow(dead_code)]
pub(crate) fn redirect(e: impl HttpError, url: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Error::new(Kind::Redirect, Some(e)).with_url(url.into()))
}

pub(crate) fn status_code(url: impl Into<String>, status: StatusCode) -> anyhow::Error {
    anyhow::Error::new(
        Error::new(Kind::Status(status), None::<hyper::http::Error>).with_url(url.into()),
    )
}
