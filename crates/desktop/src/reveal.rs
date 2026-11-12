//! Resolve selected history locations before requesting the platform file manager. Never invoke a shell.
use crate::history::HistoryEntry;
use std::{collections::BTreeSet, path::PathBuf};
pub(crate) struct Locations {
    pub paths: Vec<PathBuf>,
    pub missing: usize,
}
pub(crate) fn locations<'a>(entries: impl Iterator<Item = &'a HistoryEntry>) -> Locations {
    let mut paths = BTreeSet::new();
    let mut missing = 0;
    for entry in entries {
        let files = entry
            .artifacts
            .iter()
            .filter(|a| a.path.is_absolute() && a.path.exists())
            .map(|a| a.path.clone())
            .collect::<Vec<_>>();
        if !files.is_empty() {
            paths.extend(files);
        } else if let Some(dir) = entry
            .metadata
            .output_dir
            .as_ref()
            .filter(|p| p.is_absolute() && p.is_dir())
        {
            paths.insert(dir.clone());
        } else {
            missing += 1;
        }
    }
    Locations {
        paths: paths.into_iter().collect(),
        missing,
    }
}
pub(crate) fn reveal(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No existing file or save directory is available".into());
    }
    if paths.len() > 10 {
        return Err("Reveal at most 10 locations at once".into());
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = std::process::Command::new("/usr/bin/open");
        // One Finder selection request rather than racing individual NSWorkspace calls.
        command
            .arg("-R")
            .arg("--")
            .args(paths)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let status = command.status().map_err(|_| "Cannot launch Finder")?;
        if !status.success() {
            return Err("Finder could not reveal the selected locations".into());
        }
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        if paths.len() != 1 {
            return Err("Windows Explorer supports revealing one location at a time".into());
        }
        let path = &paths[0];
        let status = std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| "Cannot launch Windows Explorer")?;
        if status.success() {
            Ok(())
        } else {
            Err("Windows Explorer could not reveal the location".into())
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let directory = paths
            .iter()
            .find(|p| p.is_dir())
            .or_else(|| paths[0].parent())
            .ok_or("No containing directory is available")?;
        let status = std::process::Command::new("xdg-open")
            .arg(directory)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| "Cannot launch the system file manager")?;
        if status.success() {
            Ok(())
        } else {
            Err("System file manager could not open the location".into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryMetadata, HistoryStatus};
    #[test]
    fn missing_artifact_falls_back_to_existing_directory_and_deduplicates() {
        let root = std::env::temp_dir();
        let entry = HistoryEntry {
            id: "t".into(),
            title: "x".into(),
            url: "https://youtu.be/dQw4w9WgXcQ".into(),
            started_at: 0,
            finished_at: None,
            status: HistoryStatus::Succeeded,
            artifacts: vec![],
            metadata: HistoryMetadata {
                output_dir: Some(root.clone()),
                ..Default::default()
            },
        };
        let found = locations([&entry, &entry].into_iter());
        assert_eq!(found.paths, vec![root]);
        assert_eq!(found.missing, 0);
        let mut absent = entry;
        absent.metadata.output_dir = None;
        assert_eq!(locations([&absent].into_iter()).missing, 1);
    }
}
