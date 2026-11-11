//! Attempt ownership and final artifact confinement; never infer output names.
use crate::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
pub(crate) fn prepare_attempt(request: &DownloadRequest) -> Result<PathBuf> {
    fs::create_dir_all(&request.output_dir)
        .map_err(|_| EngineError::new(ErrorKind::Io, "cannot create output directory"))?;
    let root = fs::canonicalize(&request.output_dir)
        .map_err(|_| EngineError::new(ErrorKind::Io, "cannot resolve output directory"))?;
    // Exclusive attempt directory prevents cross-attempt .part collisions and
    // silent overwrite. Never derive a local path from a remote title.
    if request.attempt_id.is_empty()
        || !request
            .attempt_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err(EngineError::new(
            ErrorKind::InvalidRequest,
            "attempt_id must contain only ASCII letters, digits, _ or -",
        ));
    }
    let attempt = root.join(format!("youtube-{}", request.attempt_id));
    fs::create_dir(&attempt).map_err(|_| {
        EngineError::new(
            ErrorKind::InvalidRequest,
            "attempt output already exists or cannot be created; use a new attempt_id",
        )
    })?;
    Ok(attempt)
}
pub(crate) fn validate_artifacts(paths: Vec<PathBuf>, attempt: &Path) -> Result<DownloadResult> {
    if paths.is_empty() {
        return Err(EngineError::new(
            ErrorKind::MissingArtifact,
            "successful exit without after_move artifact",
        ));
    }
    let mut artifacts = Vec::new();
    for path in paths {
        // Require the exact final absolute path; never guess a filename.
        if !path.is_absolute() {
            return Err(EngineError::new(
                ErrorKind::UnsafeArtifact,
                "relative artifact path rejected",
            ));
        }
        let real = fs::canonicalize(&path).map_err(|_| {
            EngineError::new(
                ErrorKind::MissingArtifact,
                "reported artifact does not exist",
            )
        })?;
        if !real.starts_with(&attempt) || real == attempt {
            return Err(EngineError::new(
                ErrorKind::UnsafeArtifact,
                "artifact escapes authorized attempt directory",
            ));
        }
        let metadata = fs::metadata(&real)
            .map_err(|_| EngineError::new(ErrorKind::MissingArtifact, "cannot inspect artifact"))?;
        if !metadata.is_file() {
            return Err(EngineError::new(
                ErrorKind::UnsafeArtifact,
                "artifact is not a regular file",
            ));
        }
        if !artifacts.iter().any(|a: &Artifact| a.path == real) {
            artifacts.push(Artifact {
                path: real,
                size: metadata.len(),
            });
        }
    }
    Ok(DownloadResult { artifacts })
}
