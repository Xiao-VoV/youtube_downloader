//! Final confinement and preexisting-file detection are checks, not an OS sandbox.
use crate::*;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
pub(crate) struct Attempt {
    pub root: PathBuf,
    pub existing: HashSet<PathBuf>,
}
/// The chosen base directory may be an alias. Its generated date child must
/// be a real directory so an existing symlink cannot redirect the archive.
pub(crate) fn prepare_archive_directory(path: &Path) -> Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent)
            .map_err(|_| EngineError::new(ErrorKind::Io, "cannot create output directory"))?;
    }
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(EngineError::new(ErrorKind::Io, "cannot create date archive")),
    }
    let meta = fs::symlink_metadata(path).map_err(|_| unsafe_path())?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(unsafe_path());
    }
    Ok(())
}

pub(crate) fn prepare_attempt(request: &DownloadRequest) -> Result<Attempt> {
    fs::create_dir_all(&request.output_dir)
        .map_err(|_| EngineError::new(ErrorKind::Io, "cannot create output directory"))?;
    let root = fs::canonicalize(&request.output_dir)
        .map_err(|_| EngineError::new(ErrorKind::Io, "cannot resolve output directory"))?;
    let temp = root.join(".shiguang-tmp");
    // Multiple jobs may enter the same date folder at once.
    match private_dir(&temp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(attempt_directory_error(e)),
    }
    let meta = fs::symlink_metadata(&temp).map_err(|_| unsafe_path())?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(unsafe_path());
    }
    if request.attempt_id.is_empty()
        || !request
            .attempt_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err(EngineError::new(
            ErrorKind::InvalidRequest,
            "invalid attempt id",
        ));
    }
    private_dir(&temp.join(&request.attempt_id)).map_err(attempt_directory_error)?;
    let mut existing = HashSet::new();
    snapshot(&root, &root, &mut existing, 0)?;
    Ok(Attempt { root, existing })
}
fn private_dir(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}
fn attempt_directory_error(error: std::io::Error) -> EngineError {
    EngineError::new(
        ErrorKind::InvalidRequest,
        format!("cannot create private attempt directory: {error}"),
    )
}
fn unsafe_path() -> EngineError {
    EngineError::new(
        ErrorKind::UnsafeArtifact,
        "unsafe output directory or artifact",
    )
}
fn snapshot(root: &Path, path: &Path, seen: &mut HashSet<PathBuf>, depth: usize) -> Result<()> {
    if depth > 16 {
        return Err(unsafe_path());
    }
    for item in fs::read_dir(path).map_err(|_| unsafe_path())? {
        let path = item.map_err(|_| unsafe_path())?.path();
        if path == root.join(".shiguang-tmp") {
            continue;
        }
        if seen.len() >= 10000 {
            return Err(EngineError::new(
                ErrorKind::OutputLimit,
                "output directory snapshot exceeds limit",
            ));
        }
        let meta = fs::symlink_metadata(&path).map_err(|_| unsafe_path())?;
        // Reject symlink subdirectories before launching; do not traverse them.
        if meta.file_type().is_symlink() {
            return Err(unsafe_path());
        }
        seen.insert(path.clone());
        if meta.is_dir() {
            snapshot(root, &path, seen, depth + 1)?;
        }
    }
    Ok(())
}
pub(crate) fn validate_artifacts(paths: Vec<PathBuf>, attempt: &Attempt) -> Result<DownloadResult> {
    if paths.is_empty() {
        return Err(EngineError::new(
            ErrorKind::MissingArtifact,
            "successful exit without after_move artifact",
        ));
    }
    let mut artifacts = Vec::new();
    for path in paths {
        if !path.is_absolute() {
            return Err(unsafe_path());
        }
        let real = fs::canonicalize(&path).map_err(|_| {
            EngineError::new(
                ErrorKind::MissingArtifact,
                "reported artifact does not exist",
            )
        })?;
        if !real.starts_with(&attempt.root)
            || real == attempt.root
            || real.starts_with(attempt.root.join(".shiguang-tmp"))
        {
            return Err(unsafe_path());
        }
        // yt-dlp may report an existing file after skipping it. Never present
        // that as a new download. External concurrent creators remain a TOCTOU
        // boundary; installed tools and the destination must be trusted.
        if attempt.existing.contains(&real) {
            return Err(EngineError::new(
                ErrorKind::InvalidRequest,
                "output conflict: existing file was not downloaded",
            ));
        }
        let meta = fs::metadata(&real).map_err(|_| unsafe_path())?;
        if !meta.is_file() {
            return Err(unsafe_path());
        }
        if !artifacts.iter().any(|a: &Artifact| a.path == real) {
            artifacts.push(Artifact {
                path: real,
                size: meta.len(),
            });
        }
    }
    Ok(DownloadResult { artifacts })
}
