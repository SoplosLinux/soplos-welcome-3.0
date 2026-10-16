// Software-center-style search across apt and Flathub — separate from
// Recommended's local ~100-app catalog filter, this queries the system's
// real package databases. Both search commands build Command::args() (never
// a shell string), so the query never needs shell-injection validation —
// there's no shell to break out of.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use base64::Engine;
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub name: String,
    pub id: String,
    pub description: String,
    pub kind: String,
    pub icon: Option<String>,
}

const MAX_RESULTS: usize = 40;
const APT_LISTS_DIR: &str = "/var/lib/apt/lists";

// === apt icons: Debian's DEP-11 AppStream data, already downloaded locally
// by apt update, ships icons as JPEG XL — jxl-oxide decodes them in pure
// Rust (verified against a real icon extracted from this exact system
// before writing this) since WebKitGTK can't render JXL in an <img> tag. ===

// Decompressing every *_dep11_Components-*.yml.gz (tens of MB of YAML) and
// decoding each JXL icon is expensive enough that redoing it on every
// keystroke-triggered search made the UI stutter — both caches below are
// built at most once per process lifetime and reused for every later search.
static ICON_NAME_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
static ICON_DATA_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn icon_name_map() -> &'static HashMap<String, String> {
    ICON_NAME_MAP.get_or_init(build_dep11_icon_name_map)
}

/// Scans every *_dep11_Components-*.yml.gz once and returns a map of
/// {package name -> icon filename (e.g. "vlc_vlc.jxl")} for every package
/// that has one, so a 40-result search only decompresses each multi-MB
/// file once instead of once per package.
fn build_dep11_icon_name_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(APT_LISTS_DIR) else {
        return map;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.contains("dep11_Components") || !name.ends_with(".yml.gz") {
            continue;
        }
        let Ok(file) = File::open(&path) else { continue };
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut content = String::new();
        if decoder.read_to_string(&mut content).is_err() {
            continue;
        }
        // DEP-11 stanzas are separated by a line containing only "---".
        for stanza in content.split("\n---\n") {
            let Some(pkg_line) = stanza.lines().find(|l| l.starts_with("Package: ")) else {
                continue;
            };
            let package = pkg_line.trim_start_matches("Package: ").trim();
            if map.contains_key(package) {
                continue;
            }
            // Only the first "name:" line under the "cached:" icon block —
            // good enough, we only need one filename per package.
            if let Some(cached_idx) = stanza.find("cached:") {
                let after = &stanza[cached_idx..];
                if let Some(name_idx) = after.find("name:") {
                    let line = &after[name_idx + "name:".len()..];
                    let filename = line.lines().next().unwrap_or("").trim();
                    if !filename.is_empty() {
                        map.insert(package.to_string(), filename.to_string());
                    }
                }
            }
        }
    }
    map
}

/// Resolves the requested icon filenames to PNG data URIs, decoding each one
/// at most once per process lifetime — a cache hit skips both the tarball
/// scan and the JXL decode entirely.
fn resolve_icons(wanted: &HashSet<&str>) -> HashMap<String, String> {
    let cache = ICON_DATA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut result = HashMap::new();
    let mut missing: HashSet<&str> = HashSet::new();
    {
        let cached = cache.lock().unwrap();
        for &filename in wanted {
            match cached.get(filename) {
                Some(data_uri) => {
                    result.insert(filename.to_string(), data_uri.clone());
                }
                None => {
                    missing.insert(filename);
                }
            }
        }
    }
    if missing.is_empty() {
        return result;
    }
    let decoded = extract_and_decode_icons(&missing);
    let mut cached = cache.lock().unwrap();
    for (filename, data_uri) in decoded {
        cached.insert(filename.clone(), data_uri.clone());
        result.insert(filename, data_uri);
    }
    result
}

/// Extracts only the requested icon filenames from every *_dep11_icons-64x64.tar.gz,
/// decoding each JXL to a PNG data URI. One pass per tarball regardless of
/// how many of `wanted` filenames it actually contains.
fn extract_and_decode_icons(wanted: &HashSet<&str>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if wanted.is_empty() {
        return result;
    }
    let Ok(entries) = std::fs::read_dir(APT_LISTS_DIR) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.contains("dep11_icons-64x64.tar.gz") {
            continue;
        }
        let Ok(file) = File::open(&path) else { continue };
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        let Ok(tar_entries) = archive.entries() else { continue };
        for tar_entry in tar_entries.flatten() {
            let mut tar_entry = tar_entry;
            let Ok(entry_path) = tar_entry.path().map(|p| p.into_owned()) else { continue };
            let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if result.contains_key(filename) || !wanted.contains(filename) {
                continue;
            }
            let mut jxl_bytes = Vec::new();
            if tar_entry.read_to_end(&mut jxl_bytes).is_err() {
                continue;
            }
            if let Some(data_uri) = decode_jxl_to_png_data_uri(&jxl_bytes) {
                result.insert(filename.to_string(), data_uri);
            }
        }
    }
    result
}

fn decode_jxl_to_png_data_uri(jxl_bytes: &[u8]) -> Option<String> {
    let image = jxl_oxide::JxlImage::builder().read(jxl_bytes).ok()?;
    let render = image.render_frame(0).ok()?;
    let fb = render.image_all_channels();
    let width = fb.width() as u32;
    let height = fb.height() as u32;
    let channels = fb.channels();
    if width == 0 || height == 0 || channels == 0 {
        return None;
    }
    let buf = fb.buf();
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for px in buf.chunks(channels) {
        let r = (px[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (px.get(1).copied().unwrap_or(px[0]).clamp(0.0, 1.0) * 255.0) as u8;
        let b = (px.get(2).copied().unwrap_or(px[0]).clamp(0.0, 1.0) * 255.0) as u8;
        let a = (px.get(3).copied().unwrap_or(1.0).clamp(0.0, 1.0) * 255.0) as u8;
        rgba.extend_from_slice(&[r, g, b, a]);
    }
    let img_buf = image::RgbaImage::from_raw(width, height, rgba)?;
    let mut png_bytes = Vec::new();
    img_buf
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&png_bytes)))
}



fn search_apt_blocking(query: String) -> Vec<SearchResult> {
    let query = query.trim();
    if query.is_empty() {
        return vec![];
    }
    let output = match Command::new("apt-cache").args(["search", "--names-only", query]).output() {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    let mut results: Vec<SearchResult> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (name, description) = line.split_once(" - ")?;
            Some(SearchResult {
                name: name.trim().to_string(),
                id: name.trim().to_string(),
                description: description.trim().to_string(),
                kind: "apt".to_string(),
                icon: None,
            })
        })
        .collect();

    // Sort by relevance: exact match first, then starts-with, then contains.
    // apt-cache returns results in an arbitrary hash order; without sorting,
    // searching "obs" can show unrelated packages before obs-studio.
    let q_lower = query.to_lowercase();
    results.sort_by_key(|r| {
        let n = r.id.to_lowercase();
        if n == q_lower { 0u8 }
        else if n.starts_with(&q_lower) { 1 }
        else { 2 }
    });
    results.truncate(MAX_RESULTS);

    let name_map = icon_name_map();
    let wanted: HashSet<&str> = results.iter().filter_map(|r| name_map.get(&r.id).map(|s| s.as_str())).collect();
    let icon_data = resolve_icons(&wanted);
    for r in &mut results {
        if let Some(filename) = name_map.get(&r.id) {
            r.icon = icon_data.get(filename).cloned();
        }
    }
    results
}

fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(8)))
        .build()
        .into()
}

fn search_flathub_api(query: &str) -> Option<Vec<SearchResult>> {
    let body = serde_json::to_vec(&serde_json::json!({ "query": query })).ok()?;
    let mut response = http_agent()
        .post("https://flathub.org/api/v2/search")
        .header("Content-Type", "application/json")
        .send(&body)
        .ok()?;
    let bytes = response.body_mut().read_to_vec().ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let hits = json.get("hits")?.as_array()?;
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for hit in hits {
        let id = hit.get("app_id")?.as_str()?.to_string();
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        let name = hit.get("name").and_then(|n| n.as_str()).unwrap_or(&id).to_string();
        let description = hit.get("summary").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let icon = hit.get("icon").and_then(|i| i.as_str()).map(|s| s.to_string());
        results.push(SearchResult {
            name,
            id,
            description,
            kind: "flatpak".to_string(),
            icon,
        });
        if results.len() >= MAX_RESULTS {
            break;
        }
    }
    Some(results)
}

fn search_flatpak_blocking(query: String) -> Vec<SearchResult> {
    let query = query.trim();
    if query.is_empty() {
        return vec![];
    }
    if let Some(results) = search_flathub_api(query) {
        if !results.is_empty() {
            return results;
        }
    }
    let output = match Command::new("flatpak")
        .args(["search", query, "--columns=name,application,description"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    let mut seen = HashSet::new();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let name = parts.next()?.trim().to_string();
            let id = parts.next()?.trim().to_string();
            let description = parts.next().unwrap_or("").trim().to_string();
            if name.is_empty() || id.is_empty() || id == "-" || !seen.insert(id.clone()) {
                return None;
            }
            Some(SearchResult {
                name,
                id,
                description,
                kind: "flatpak".to_string(),
                icon: None,
            })
        })
        .take(MAX_RESULTS)
        .collect()
}

fn search_snap_store(query: &str) -> Option<Vec<SearchResult>> {
    let encoded: String = query
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    let url = format!("https://api.snapcraft.io/v2/snaps/find?q={encoded}&fields=media,summary,title");
    let mut response = http_agent().get(&url).header("Snap-Device-Series", "16").call().ok()?;
    let bytes = response.body_mut().read_to_vec().ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let results_arr = json.get("results")?.as_array()?;
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for item in results_arr {
        let name = item.get("name")?.as_str()?.to_string();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        let snap = item.get("snap");
        let title = snap
            .and_then(|s| s.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or(&name)
            .to_string();
        let description = snap
            .and_then(|s| s.get("summary"))
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let icon = snap
            .and_then(|s| s.get("media"))
            .and_then(|m| m.as_array())
            .and_then(|arr| {
                arr.iter().find(|m| m.get("type").and_then(|t| t.as_str()) == Some("icon"))
            })
            .and_then(|m| m.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        results.push(SearchResult {
            name: title,
            id: name,
            description,
            kind: "snap".to_string(),
            icon,
        });
        if results.len() >= MAX_RESULTS {
            break;
        }
    }
    Some(results)
}

fn search_snap_blocking(query: String) -> Vec<SearchResult> {
    let query = query.trim();
    if query.is_empty() {
        return vec![];
    }
    if let Some(results) = search_snap_store(query) {
        if !results.is_empty() {
            return results;
        }
    }
    let output = match Command::new("snap").args(["find", query]).output() {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    let mut seen = HashSet::new();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1) // header row
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?.to_string();
            parts.next()?; // version
            parts.next()?; // publisher
            parts.next()?; // notes
            let description = parts.collect::<Vec<_>>().join(" ");
            if name.is_empty() || !seen.insert(name.clone()) {
                return None;
            }
            Some(SearchResult { name: name.clone(), id: name, description, kind: "snap".to_string(), icon: None })
        })
        .take(MAX_RESULTS)
        .collect()
}

// Tauri runs plain `fn` commands on the main thread, so these used to freeze
// the whole UI (including the progress bar animation) for the entire search —
// and the very first apt search also builds the DEP-11 icon index, which
// decompresses tens of MB. Running them on the blocking pool keeps the window
// responsive and lets the three searches actually run in parallel.
macro_rules! async_search {
    ($name:ident, $inner:ident) => {
        #[tauri::command]
        pub async fn $name(query: String) -> Vec<SearchResult> {
            tauri::async_runtime::spawn_blocking(move || $inner(query))
                .await
                .unwrap_or_default()
        }
    };
}

async_search!(search_apt, search_apt_blocking);
async_search!(search_flatpak, search_flatpak_blocking);
async_search!(search_snap, search_snap_blocking);

/// Builds the apt icon index in the background so the first real search
/// doesn't pay for it.
#[tauri::command]
pub async fn prewarm_search() {
    let _ = tauri::async_runtime::spawn_blocking(|| {
        icon_name_map();
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_speeds() {
        let t0 = std::time::Instant::now();
        let apt = search_apt_blocking("obs".to_string());
        let t_apt = t0.elapsed();

        let t1 = std::time::Instant::now();
        let flatpak = search_flatpak_blocking("obs".to_string());
        let t_flatpak = t1.elapsed();

        let t2 = std::time::Instant::now();
        let snap = search_snap_blocking("obs".to_string());
        let t_snap = t2.elapsed();

        println!("APT ({} results): {:?}", apt.len(), t_apt);
        println!("Flatpak ({} results): {:?}", flatpak.len(), t_flatpak);
        println!("Snap ({} results): {:?}", snap.len(), t_snap);
    }
}
