//! Custom validators used by the request bodies in `models`.

use std::borrow::Cow;

use validator::ValidationError;

/// Paths the router already uses, so they can't be claimed as short codes.
const RESERVED_CODES: &[&str] = &[
    "api",
    "assets",
    "docs",
    "favicon.svg",
    "health",
    "openapi.json",
];

fn error(code: &'static str, message: &'static str) -> ValidationError {
    ValidationError::new(code).with_message(Cow::Borrowed(message))
}

pub fn http_url(value: &str) -> Result<(), ValidationError> {
    let url = url::Url::parse(value).map_err(|_| error("url", "must be a valid URL"))?;
    match url.scheme() {
        "http" | "https" if url.host().is_some() => Ok(()),
        _ => Err(error("url", "must be an http(s) URL with a host")),
    }
}

pub fn short_code(value: &str) -> Result<(), ValidationError> {
    if !(3..=32).contains(&value.len()) {
        return Err(error("code", "must be between 3 and 32 characters"));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(error(
            "code",
            "may only contain letters, digits, '-' and '_'",
        ));
    }
    if RESERVED_CODES.contains(&value.to_ascii_lowercase().as_str()) {
        return Err(error("code", "is reserved"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_http_and_https_urls() {
        assert!(http_url("https://example.com/path?q=1").is_ok());
        assert!(http_url("http://localhost:3000").is_ok());
    }

    #[test]
    fn rejects_other_schemes_and_garbage() {
        assert!(http_url("ftp://example.com").is_err());
        assert!(http_url("javascript:alert(1)").is_err());
        assert!(http_url("not a url").is_err());
    }

    #[test]
    fn validates_short_codes() {
        assert!(short_code("rust-book_2").is_ok());
        assert!(short_code("ab").is_err());
        assert!(short_code("has space").is_err());
        assert!(short_code("DOCS").is_err());
        assert!(short_code(&"x".repeat(33)).is_err());
    }
}
