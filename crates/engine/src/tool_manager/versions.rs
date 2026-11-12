//! Explicit version selection/removal. Never execute binaries or recursively delete.
use super::*;
#[derive(Clone, Debug)]
pub struct ManagedVersion {
    pub version: String,
    pub path: PathBuf,
    pub active: bool,
}
impl ToolManager {
    /// Validate the immutable layout before presenting or mutating a version.
    fn checked_version(&self, version: &str) -> ManagerResult<PathBuf> {
        valid_version(version)?;
        let binary = self.binary(version);
        self.safe_path(&binary)?;
        let directory = binary
            .parent()
            .ok_or_else(|| failure("invalid version directory"))?;
        let mut count = 0;
        for item in fs::read_dir(directory).map_err(|_| failure("version directory unavailable"))? {
            let item = item.map_err(|_| failure("cannot inspect version"))?;
            count += 1;
            if item.file_name() != asset()
                || !item
                    .file_type()
                    .map_err(|_| failure("cannot inspect version file"))?
                    .is_file()
            {
                return Err(failure("version contains unknown files or symlinks"));
            }
        }
        if count != 1 {
            return Err(failure("version binary missing"));
        }
        resolve_tool(&binary).map_err(|_| failure("managed binary is not executable"))
    }
    pub fn installed_versions(&self) -> ManagerResult<Vec<ManagedVersion>> {
        let marker = self.marker()?;
        let root = self.root.join("versions");
        self.safe_path(&root)?;
        let directory = match fs::read_dir(&root) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(failure("cannot list managed versions")),
        };
        let mut versions = Vec::new();
        for item in directory {
            if versions.len() >= 128 {
                return Err(failure("managed version limit exceeded"));
            }
            let item = item.map_err(|_| failure("cannot list version"))?;
            let version = item
                .file_name()
                .into_string()
                .map_err(|_| failure("invalid version name"))?;
            let path = self.checked_version(&version)?;
            let active = marker.as_ref().is_some_and(|m| m.active == version);
            versions.push(ManagedVersion {
                version,
                path,
                active,
            });
        }
        versions.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(versions)
    }
    pub fn activate(&self, version: &str) -> ManagerResult<PathBuf> {
        let path = self.checked_version(version)?;
        let old = self.marker()?;
        if old.as_ref().is_some_and(|m| m.active == version) {
            return Ok(path);
        }
        self.write_marker(&Marker {
            active: version.into(),
            previous: old.map(|m| m.active),
        })?;
        Ok(path)
    }
    pub fn remove_version(&self, version: &str, protected_paths: &[PathBuf]) -> ManagerResult<()> {
        let path = self.checked_version(version)?;
        let marker = self.marker()?;
        if marker.as_ref().is_some_and(|m| m.active == version) {
            return Err(failure("cannot remove active managed version"));
        }
        for protected in protected_paths {
            let protected = fs::canonicalize(protected)
                .map_err(|_| failure("cannot verify protected tool path"))?;
            if protected == path {
                return Err(failure(
                    "version is protected by an active task or selection",
                ));
            }
        }
        // checked_version verifies exactly one regular executable. Recheck before
        // deleting the exact file, then remove only its now-empty directory.
        self.checked_version(version)?;
        if marker
            .as_ref()
            .is_some_and(|m| m.previous.as_deref() == Some(version))
        {
            let mut marker = marker.unwrap();
            marker.previous = None;
            self.write_marker(&marker)?;
        }
        fs::remove_file(self.binary(version))
            .map_err(|_| failure("cannot remove version executable"))?;
        fs::remove_dir(self.binary(version).parent().unwrap())
            .map_err(|_| failure("cannot remove nonempty version directory"))?;
        Ok(())
    }
}
