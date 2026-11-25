//! "A newer Strata is out" check against the project's GitHub releases.

use serde::Serialize;

pub const REPO: &str = "ratinhogauchonoob-arch/strata";

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct UpdateInfo {
    pub available: bool,
    pub current: String,
    pub latest: String,
    pub url: String,
    pub prerelease: bool,
}

/// "v0.10.0-beta" → (0, 10, 0). Anything after the numbers is ignored.
pub fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(|c: char| !(c.is_ascii_digit() || c == '.')).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u32>());
    let a = it.next()?.ok()?;
    let b = it.next().unwrap_or(Ok(0)).ok()?;
    let c = it.next().unwrap_or(Ok(0)).ok()?;
    Some((a, b, c))
}

/// Picks the newest non-draft release from the GitHub list (prereleases count: Strata is a beta).
pub fn pick(current: &str, list: &serde_json::Value) -> UpdateInfo {
    let mut out = UpdateInfo { current: current.into(), ..Default::default() };
    let cur = parse_version(current);
    let best = list.as_array().into_iter().flatten()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| parse_version(r["tag_name"].as_str().unwrap_or("")).map(|v| (v, r)))
        .max_by_key(|(v, _)| *v);
    if let Some((v, r)) = best {
        out.latest = r["tag_name"].as_str().unwrap_or("").to_string();
        out.url = r["html_url"].as_str().unwrap_or("").to_string();
        out.prerelease = r["prerelease"].as_bool().unwrap_or(false);
        out.available = cur.map(|c| v > c).unwrap_or(false);
    }
    out
}

pub fn check() -> Result<UpdateInfo, String> {
    let agent = ureq::AgentBuilder::new()
        .user_agent(&format!("Strata/{}", crate::APP_VERSION))
        .timeout(std::time::Duration::from_secs(8))
        .build();
    let list: serde_json::Value = agent
        .get(&format!("https://api.github.com/repos/{REPO}/releases?per_page=10"))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
        .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()))?;
    Ok(pick(crate::APP_VERSION, &list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.10.0-beta"), Some((0, 10, 0)));
        assert_eq!(parse_version("0.10.1"), Some((0, 10, 1)));
        assert_eq!(parse_version("v1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("build-12"), None);
    }

    #[test]
    fn picks_newer_release() {
        let list = json!([
            { "tag_name": "v0.11.0-beta", "draft": true, "html_url": "d", "prerelease": true },
            { "tag_name": "v0.10.2-beta", "draft": false, "html_url": "https://x/0.10.2", "prerelease": true },
            { "tag_name": "v0.10.0-beta", "draft": false, "html_url": "https://x/0.10.0", "prerelease": true },
        ]);
        let u = pick("0.10.1", &list);
        assert!(u.available);
        assert_eq!(u.latest, "v0.10.2-beta");
        assert_eq!(u.url, "https://x/0.10.2");
        assert!(!pick("0.10.2", &list).available);
        assert!(!pick("0.10.1", &json!([])).available);
    }
}
