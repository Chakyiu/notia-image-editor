//! Resolve an embedded `![](path)` or wiki image link to a vault-relative path.

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

/// Whether `path` looks like a vault image attachment (not a note).
pub fn is_supported_image_path(path: &str) -> bool {
    let path = path.trim();
    if path.is_empty() || path.contains("://") {
        return false;
    }
    let ext = path
        .rsplit('.')
        .next()
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    IMAGE_EXTENSIONS.iter().any(|known| ext == *known)
}

/// Parse a markdown image embed on a line, e.g. `![](attachments/photo.png)`.
pub fn image_target_from_line(line: &str) -> Option<String> {
    let line = line.trim();
    let start = line.find("![").or_else(|| line.find("![["))?;
    let rest = &line[start..];
    let path = if rest.starts_with("![[") {
        let inner = rest.strip_prefix("![[")?;
        let end = inner.find("]]")?;
        inner[..end].split('|').next()?.trim()
    } else {
        let paren = rest.find("](").or_else(|| rest.find('('))?;
        let open = rest[paren..].find('(')? + paren + 1;
        let close = rest[open..].find(')')? + open;
        rest[open..close].trim()
    };
    let path = path.trim_matches('"').trim_matches('\'');
    if is_supported_image_path(path) {
        Some(path.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_image() {
        assert_eq!(
            image_target_from_line("  ![](attachments/pic.png) rest"),
            Some("attachments/pic.png".into())
        );
    }

    #[test]
    fn rejects_notes() {
        assert!(!is_supported_image_path("Notes/Daily"));
    }
}
