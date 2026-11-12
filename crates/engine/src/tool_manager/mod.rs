//! Explicit, blocking tool management. Call from user-initiated workers only.
//! Official same-origin SHA256 files provide integrity, not signature trust.
#[cfg(test)]
mod tests;
mod transport;
mod versions;
use crate::{ToolPaths, resolve_tool};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use transport::{HttpsTransport, Transport};
pub use versions::ManagedVersion;
pub type ManagerResult<T> = std::result::Result<T, String>;
fn failure(message: &str) -> String {
    format!("Tool manager: {message}")
}
static TEMP: AtomicU64 = AtomicU64::new(0);
const RELEASE_API: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
const RELEASE_ROOT: &str = "https://github.com/yt-dlp/yt-dlp/releases/download";
fn asset() -> &'static str {
    crate::platform::yt_dlp_asset()
}
#[derive(Clone, Debug, Default)]
pub struct CustomToolPaths {
    pub yt_dlp: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    pub deno: Option<PathBuf>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolSource {
    Custom,
    Managed,
    Path,
}
#[derive(Clone, Debug)]
pub struct ResolvedTools {
    pub paths: ToolPaths,
    pub yt_dlp_source: ToolSource,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub asset_url: String,
    pub checksum_url: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Marker {
    active: String,
    previous: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ToolManager {
    root: PathBuf,
}
impl ToolManager {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn default_root() -> ManagerResult<PathBuf> {
        crate::platform::app_data_dir()
            .map(|path| path.join("tools"))
            .map_err(|error| failure(&error))
    }
    pub fn resolve(&self, custom: &CustomToolPaths) -> ManagerResult<ResolvedTools> {
        let (yt_dlp, yt_dlp_source) = if let Some(path) = &custom.yt_dlp {
            (
                resolve_tool(path).map_err(|_| failure("invalid custom yt-dlp"))?,
                ToolSource::Custom,
            )
        } else if let Some(marker) = self.marker()? {
            self.safe_path(&self.binary(&marker.active))?;
            (
                resolve_tool(&self.binary(&marker.active))
                    .map_err(|_| failure("managed executable missing; select custom or revert"))?,
                ToolSource::Managed,
            )
        } else {
            (
                resolve_tool(Path::new("yt-dlp")).map_err(|_| failure("yt-dlp not found"))?,
                ToolSource::Path,
            )
        };
        let optional = |custom: &Option<PathBuf>, name: &str| -> ManagerResult<Option<PathBuf>> {
            match custom {
                Some(path) => resolve_tool(path)
                    .map(Some)
                    .map_err(|_| failure("invalid custom media tool")),
                None => Ok(resolve_tool(Path::new(name)).ok()),
            }
        };
        Ok(ResolvedTools {
            paths: ToolPaths {
                yt_dlp,
                ffmpeg: optional(&custom.ffmpeg, "ffmpeg")?,
                ffprobe: optional(&custom.ffprobe, "ffprobe")?,
                deno: optional(&custom.deno, "deno")?,
            },
            yt_dlp_source,
        })
    }
    pub fn check_release(&self) -> ManagerResult<ReleaseInfo> {
        self.check_with(&HttpsTransport)
    }
    pub fn install(&self, release: &ReleaseInfo) -> ManagerResult<PathBuf> {
        if !cfg!(target_os = "macos") {
            return Err(failure("managed install supports macOS only"));
        }
        self.install_with(release, &HttpsTransport)
    }
    /// Download and verify a version without selecting it for future tasks.
    pub fn install_only(&self, release: &ReleaseInfo) -> ManagerResult<PathBuf> {
        if !cfg!(target_os = "macos") {
            return Err(failure("managed install supports macOS only"));
        }
        self.install_only_with(release, &HttpsTransport)
    }
    pub fn revert(&self) -> ManagerResult<PathBuf> {
        let marker = self
            .marker()?
            .ok_or_else(|| failure("no managed installation"))?;
        let previous = marker
            .previous
            .ok_or_else(|| failure("no previous version"))?;
        self.safe_path(&self.binary(&previous))?;
        let path = resolve_tool(&self.binary(&previous))
            .map_err(|_| failure("previous version missing"))?;
        self.write_marker(&Marker {
            active: previous,
            previous: Some(marker.active),
        })?;
        Ok(path)
    }
    fn check_with(&self, transport: &impl Transport) -> ManagerResult<ReleaseInfo> {
        let bytes = transport.get(RELEASE_API, 1024 * 1024)?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| failure("invalid release JSON"))?;
        let version = value
            .get("tag_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| failure("release tag missing"))?;
        release_info(version)
    }
    fn binary(&self, version: &str) -> PathBuf {
        self.root.join("versions").join(version).join(asset())
    }
    /// Reject managed-root aliases and every existing descendant symlink.
    /// Ancestors outside this root are caller-owned; this is not an OS sandbox
    /// against concurrent filesystem replacement by another local process.
    fn safe_path(&self, path: &Path) -> ManagerResult<()> {
        if !self.root.is_absolute()
            || self.root.parent().is_none()
            || self.root.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            return Err(failure(
                "managed root must be a non-root absolute directory",
            ));
        }
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| failure("managed path escapes root"))?;
        let mut current = self.root.clone();
        let mut paths = vec![current.clone()];
        for component in relative.components() {
            current.push(component);
            paths.push(current.clone());
        }
        for current in paths {
            match fs::symlink_metadata(&current) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    return Err(failure("managed symlink rejected"));
                }
                Ok(meta) if current != path && !meta.is_dir() => {
                    return Err(failure("managed parent is not a directory"));
                }
                Ok(meta) if current == self.root && !meta.is_dir() => {
                    return Err(failure("managed root is not a directory"));
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(failure("cannot inspect managed path")),
            }
        }
        Ok(())
    }
    fn marker(&self) -> ManagerResult<Option<Marker>> {
        let path = self.root.join("active.json");
        self.safe_path(&path)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(failure("marker unreadable")),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 4096 {
            return Err(failure("unsafe active marker"));
        }
        let marker: Marker =
            serde_json::from_slice(&fs::read(path).map_err(|_| failure("marker unreadable"))?)
                .map_err(|_| failure("invalid active marker"))?;
        valid_version(&marker.active)?;
        if let Some(version) = &marker.previous {
            valid_version(version)?;
        }
        Ok(Some(marker))
    }
    fn write_marker(&self, marker: &Marker) -> ManagerResult<()> {
        self.safe_path(&self.root.join("active.json"))?;
        atomic_file(
            &self.root.join("active.json"),
            &serde_json::to_vec(marker).map_err(|_| failure("marker encoding failed"))?,
            false,
        )
    }
    fn install_with(
        &self,
        release: &ReleaseInfo,
        transport: &impl Transport,
    ) -> ManagerResult<PathBuf> {
        let old = self.marker()?;
        let binary = self.install_only_with(release, transport)?;
        self.write_marker(&Marker {
            active: release.version.clone(),
            previous: old.map(|m| m.active),
        })?;
        Ok(binary)
    }
    fn install_only_with(
        &self,
        release: &ReleaseInfo,
        transport: &impl Transport,
    ) -> ManagerResult<PathBuf> {
        let expected = release_info(&release.version)?;
        if release.asset_url != expected.asset_url || release.checksum_url != expected.checksum_url
        {
            return Err(failure("release URLs are not official fixed assets"));
        }
        self.safe_path(&self.binary(&release.version))?;
        let old = self.marker()?;
        if old.as_ref().is_some_and(|m| m.active == release.version) {
            return Err(failure("version already active"));
        }
        let sums = transport.get(&release.checksum_url, 1024 * 1024)?;
        let digest = checksum(&sums)?;
        let bytes = transport.get(&release.asset_url, 64 * 1024 * 1024)?;
        if bytes.is_empty() || format!("{:x}", Sha256::digest(&bytes)) != digest {
            return Err(failure("release SHA256 mismatch"));
        }
        // Never replace an installed version; a failed stage leaves active unchanged.
        self.safe_path(&self.binary(&release.version))?;
        let directory = self.binary(&release.version).parent().unwrap().to_owned();
        fs::create_dir_all(directory.parent().unwrap())
            .map_err(|_| failure("cannot create version root"))?;
        fs::create_dir(&directory)
            .map_err(|_| failure("version directory exists or cannot be created"))?;
        let binary = self.binary(&release.version);
        atomic_file(&binary, &bytes, true)?;
        Ok(binary)
    }
}
fn valid_version(version: &str) -> ManagerResult<()> {
    if version.is_empty()
        || version.len() > 64
        || !version.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        || version.split('.').any(str::is_empty)
    {
        return Err(failure("unsupported release version"));
    }
    Ok(())
}
fn release_info(version: &str) -> ManagerResult<ReleaseInfo> {
    valid_version(version)?;
    Ok(ReleaseInfo {
        version: version.into(),
        asset_url: format!("{RELEASE_ROOT}/{version}/{}", asset()),
        checksum_url: format!("{RELEASE_ROOT}/{version}/SHA2-256SUMS"),
    })
}
fn checksum(bytes: &[u8]) -> ManagerResult<String> {
    let text = std::str::from_utf8(bytes).map_err(|_| failure("invalid checksum UTF8"))?;
    let mut result = None;
    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() == 2 && fields[1].trim_start_matches('*') == asset() {
            if result.is_some()
                || fields[0].len() != 64
                || !fields[0].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(failure("ambiguous checksum"));
            }
            result = Some(fields[0].to_ascii_lowercase());
        }
    }
    result.ok_or_else(|| failure("selected platform checksum missing"))
}
fn atomic_file(path: &Path, bytes: &[u8], executable: bool) -> ManagerResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| failure("invalid managed path"))?;
    fs::create_dir_all(parent).map_err(|_| failure("cannot create managed directory"))?;
    let temporary = parent.join(format!(
        ".stage-{}-{}",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if executable { 0o700 } else { 0o600 });
    }
    let mut file = options
        .open(&temporary)
        .map_err(|_| failure("cannot create stage"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| failure("cannot write stage"))?;
    fs::rename(&temporary, path).map_err(|_| failure("cannot activate stage"))?;
    Ok(())
}
