pub(crate) fn normalize_path(path: impl AsRef<str>) -> String {
    let trimmed = path.as_ref().trim();
    if trimmed.is_empty() {
        return "/".to_string();
    }

    let with_leading = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };

    if with_leading.len() > 1 && with_leading.ends_with('/') {
        with_leading.trim_end_matches('/').to_string()
    } else {
        with_leading
    }
}

pub(crate) fn path_has_prefix_segment(path: &str, prefix: &str) -> bool {
    if prefix == "/" {
        return true;
    }

    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_handles_empty_whitespace_and_missing_slash() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("   "), "/");
        assert_eq!(normalize_path("api"), "/api");
    }

    #[test]
    fn normalize_path_removes_trailing_slash_except_root() {
        assert_eq!(normalize_path("/api/"), "/api");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn prefix_matching_is_segment_aware() {
        assert!(path_has_prefix_segment("/panel", "/panel"));
        assert!(path_has_prefix_segment("/panel/api", "/panel"));
        assert!(!path_has_prefix_segment("/panelized", "/panel"));
        assert!(path_has_prefix_segment("/anything", "/"));
    }
}
