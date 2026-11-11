//! Fixed-origin HTTPS transport. Checksums establish integrity, not signatures.
use super::{failure, ManagerResult};
use std::{io::Read, time::Duration};
pub(super) trait Transport {
    fn get(&self, url: &str, limit: usize) -> ManagerResult<Vec<u8>>;
}
pub(super) struct HttpsTransport;
pub(super) fn allowed(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
impl Transport for HttpsTransport {
    fn get(&self, url: &str, limit: usize) -> ManagerResult<Vec<u8>> {
        let parsed = url::Url::parse(url).map_err(|_| failure("invalid download URL"))?;
        if !allowed(&parsed) {
            return Err(failure("download host not allowed"));
        }
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(15))
            .user_agent("Shiguang-tool-manager")
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 || !allowed(attempt.url()) {
                    attempt.error("untrusted redirect")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|_| failure("cannot initialize HTTPS transport"))?;
        let response = client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|_| failure("official release request failed"))?;
        if response.content_length().is_some_and(|n| n > limit as u64) {
            return Err(failure("release exceeds size limit"));
        }
        let mut bytes = Vec::new();
        response
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| failure("release read failed"))?;
        if bytes.len() > limit {
            return Err(failure("release exceeds size limit"));
        }
        Ok(bytes)
    }
}
