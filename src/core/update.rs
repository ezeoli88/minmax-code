use reqwest::Client;

/// Check for a newer release on GitHub.
/// Returns `Some("x.y.z")` if a newer version exists, `None` otherwise.
pub async fn check_for_update() -> Option<String> {
    let current = env!("CARGO_PKG_VERSION");
    let client = Client::new();
    let resp = client
        .get("https://api.github.com/repos/ezeoli88/minmax-code/releases/latest")
        .header("User-Agent", format!("minmax-code/{}", current))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    let tag = body.get("tag_name")?.as_str()?;
    let remote = tag.strip_prefix('v').unwrap_or(tag);

    if is_newer(remote, current) {
        Some(remote.to_string())
    } else {
        None
    }
}

/// Returns true if `remote` is strictly newer than `local` (semver).
fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |v: &str| -> (u64, u64, u64) {
        let parts: Vec<u64> = v.split('.').filter_map(|s| s.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    parse(remote) > parse(local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_major() {
        assert!(is_newer("1.0.0", "0.1.0"));
    }

    #[test]
    fn newer_minor() {
        assert!(is_newer("0.2.0", "0.1.0"));
    }

    #[test]
    fn newer_patch() {
        assert!(is_newer("0.1.1", "0.1.0"));
    }

    #[test]
    fn same_version() {
        assert!(!is_newer("0.1.0", "0.1.0"));
    }

    #[test]
    fn older_version() {
        assert!(!is_newer("0.0.9", "0.1.0"));
    }
}
