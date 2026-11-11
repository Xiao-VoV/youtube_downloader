//! Deliberately tiny template grammar, not yt-dlp's expression language.
use crate::*;
fn invalid() -> EngineError {
    EngineError::new(
        ErrorKind::InvalidRequest,
        "unsafe filename or folder template",
    )
}
pub(crate) fn output_template(profile: &DownloadProfile) -> Result<String> {
    let suffix = ".%(ext)s";
    let filename = profile
        .filename_template
        .strip_suffix(suffix)
        .ok_or_else(invalid)?;
    validate(filename, false)?;
    if let Some(folder) = &profile.folder_template {
        validate(folder, true)?;
        Ok(format!("{folder}/{}", profile.filename_template))
    } else {
        Ok(profile.filename_template.clone())
    }
}
fn validate(template: &str, folders: bool) -> Result<()> {
    if template.is_empty()
        || template.len() > 512
        || template.starts_with('/')
        || template.contains('\\')
        || template.chars().any(char::is_control)
    {
        return Err(invalid());
    }
    for segment in template.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.starts_with('.')
            || segment.contains(':')
            || (!folders && template.contains('/'))
        {
            return Err(invalid());
        }
    }
    let mut remaining = template;
    while let Some(at) = remaining.find('%') {
        remaining = &remaining[at..];
        let token = [
            "%(title)s",
            "%(title).180B",
            "%(id)s",
            "%(uploader)s",
            "%(channel)s",
            "%(extractor)s",
        ]
        .into_iter()
        .find(|token| remaining.starts_with(token))
        .ok_or_else(invalid)?;
        remaining = &remaining[token.len()..];
    }
    Ok(())
}
