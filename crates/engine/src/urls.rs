use crate::{EngineError, ErrorKind, Result};
use url::Url;

/// Accept one public HTTP(S) media URL without shell or credential data.
pub fn normalize_url(input: &str) -> Result<String> {
    let bad = || {
        EngineError::new(
            ErrorKind::InvalidUrl,
            "expected one public HTTP(S) media URL",
        )
    };
    if input.chars().any(|c| c.is_control()) {
        return Err(bad());
    }
    let u = Url::parse(input.trim()).map_err(|_| bad())?;
    if !matches!(u.scheme(), "http" | "https")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
    {
        return Err(bad());
    }
    let _host = u.host_str().ok_or_else(bad)?;
    Ok(u.to_string())
}
