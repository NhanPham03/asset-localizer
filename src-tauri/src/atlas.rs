use std::collections::HashMap;
use std::path::{Path, PathBuf};

use encoding_rs::GB18030;
use serde::{Deserialize, Serialize};

/// Read atlas/plist text; Spine/Cocos exports on Windows are often GBK/GB18030, not UTF-8.
fn read_config_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("Failed to read \"{}\": {err}", path.display()))?;
    decode_config_bytes(&bytes)
}

fn decode_config_bytes(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16_le(&bytes[2..]);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16_be(&bytes[2..]);
    }

    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };

    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.to_string());
    }

    let (decoded, _, _) = GB18030.decode(bytes);
    Ok(decoded.into_owned())
}

fn decode_utf16_le(bytes: &[u8]) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err("Invalid UTF-16 LE text (odd byte length)".to_string());
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    String::from_utf16(&units).map_err(|err| format!("Invalid UTF-16 LE text: {err}"))
}

fn decode_utf16_be(bytes: &[u8]) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err("Invalid UTF-16 BE text (odd byte length)".to_string());
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();
    String::from_utf16(&units).map_err(|err| format!("Invalid UTF-16 BE text: {err}"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtlasRegion {
    pub name: String,
    /// Left edge in texture pixels (same top-left origin as the PNG/canvas).
    pub x: i32,
    /// Top edge in texture pixels (same top-left origin as the PNG/canvas).
    pub y: i32,
    /// Packed axis-aligned width in the texture.
    pub width: i32,
    /// Packed axis-aligned height in the texture.
    pub height: i32,
    /// Region was packed rotated 90° CCW (LibGDX) / CW (Cocos) in the atlas.
    pub rotated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct AtlasInfo {
    pub config_path: String,
    pub config_type: String,
    pub texture_file_name: Option<String>,
    pub texture_width: Option<u32>,
    pub texture_height: Option<u32>,
    pub regions: Vec<AtlasRegion>,
}

/// Index of atlas/plist configs keyed by referenced texture file name.
#[derive(Clone, Default)]
pub struct AtlasIndex {
    by_texture_name: HashMap<String, Vec<(PathBuf, AtlasInfo)>>,
}

impl AtlasIndex {
    pub fn lookup(&self, image_path: &Path) -> Option<AtlasInfo> {
        let file_name = image_path.file_name()?.to_string_lossy();
        let file_key = texture_key(&file_name);
        let stem_key = image_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let image_dir = image_path.parent()?;

        for key in [file_key, stem_key] {
            if key.is_empty() {
                continue;
            }
            if let Some(info) = self.lookup_key(key, image_dir) {
                return Some(info);
            }
        }
        None
    }

    fn lookup_key(&self, key: String, image_dir: &Path) -> Option<AtlasInfo> {
        let candidates = self.by_texture_name.get(&key)?;
        if let Some((_, info)) = candidates.iter().find(|(dir, _)| dir == image_dir) {
            return Some(info.clone());
        }
        candidates.first().map(|(_, info)| info.clone())
    }
}

fn texture_key(name: &str) -> String {
    Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name)
        .to_lowercase()
}

fn collect_config_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|err| format!("Failed to read directory \"{}\": {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in \"{}\": {err}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_config_files(&path, files)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("plist") || ext.eq_ignore_ascii_case("atlas") {
                files.push(path);
            }
        }
    }

    Ok(())
}

pub fn build_atlas_index(root: &Path) -> Result<AtlasIndex, String> {
    let mut files = Vec::new();
    collect_config_files(root, &mut files)?;

    let mut index = AtlasIndex::default();

    for config_path in files {
        let config_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let info = if config_path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("plist"))
        {
            parse_cocos_plist(&config_path).ok()
        } else {
            parse_libgdx_atlas(&config_path).ok()
        };

        let Some(mut info) = info else { continue };
        if info.regions.is_empty() {
            continue;
        }

        if info.texture_file_name.is_none() {
            if let Some(stem) = config_path.file_stem() {
                info.texture_file_name = Some(format!("{}.png", stem.to_string_lossy()));
            }
        }

        for key in atlas_index_keys(&config_path, &info) {
            index
                .by_texture_name
                .entry(key)
                .or_default()
                .push((config_dir.clone(), info.clone()));
        }
    }

    Ok(index)
}

fn atlas_index_keys(config_path: &Path, info: &AtlasInfo) -> Vec<String> {
    let mut keys = Vec::new();

    if let Some(ref texture) = info.texture_file_name {
        keys.push(texture_key(texture));
        if let Some(stem) = Path::new(texture).file_stem() {
            keys.push(stem.to_string_lossy().to_lowercase());
        }
    }

    if let Some(stem) = config_path.file_stem() {
        let stem = stem.to_string_lossy().to_lowercase();
        keys.push(stem.clone());
        keys.push(format!("{stem}.png"));
        keys.push(format!("{stem}.jpg"));
        keys.push(format!("{stem}.webp"));
    }

    keys.sort();
    keys.dedup();
    keys
}

pub fn atlas_for_image_with_index(image_path: &Path, index: &AtlasIndex) -> Option<AtlasInfo> {
    let dir = image_path.parent()?;
    let stem = image_path.file_stem()?.to_string_lossy();

    let atlas_path = dir.join(format!("{stem}.atlas"));
    if atlas_path.is_file() {
        if let Ok(info) = parse_libgdx_atlas(&atlas_path) {
            if !info.regions.is_empty() {
                return Some(info);
            }
        }
    }

    let plist_path = dir.join(format!("{stem}.plist"));
    if plist_path.is_file() {
        if let Ok(info) = parse_cocos_plist(&plist_path) {
            if !info.regions.is_empty() {
                return Some(info);
            }
        }
    }

    if let Some(info) = index.lookup(image_path) {
        return Some(info);
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        let image_name = image_path.file_name()?.to_string_lossy();
        let image_key = texture_key(&image_name);
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            let info = if ext.eq_ignore_ascii_case("plist") {
                parse_cocos_plist(&path).ok()
            } else if ext.eq_ignore_ascii_case("atlas") {
                parse_libgdx_atlas(&path).ok()
            } else {
                None
            };
            if let Some(info) = info {
                if info.regions.is_empty() {
                    continue;
                }
                let plist_stem = path.file_stem().map(|s| s.to_string_lossy().to_lowercase());
                let image_stem = image_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_lowercase());
                if plist_stem.is_some() && plist_stem == image_stem {
                    return Some(info);
                }
                if info
                    .texture_file_name
                    .as_ref()
                    .is_some_and(|name| texture_key(name) == image_key)
                {
                    return Some(info);
                }
            }
        }
    }

    None
}

// --- Plist coordinate normalization ---

#[derive(Clone, Copy)]
struct PlistPack {
    format: i32,
    from_frame: bool,
}

fn normalize_plist_rect(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rotated: bool,
    pack: PlistPack,
    page_height: Option<i32>,
) -> Option<(i32, i32, i32, i32)> {
    if w <= 0 || h <= 0 {
        return None;
    }

    let swap = if pack.from_frame { rotated } else { false };
    let (tw, th) = if swap { (h, w) } else { (w, h) };

    let ty = if pack.from_frame && pack.format <= 1 {
        page_height? - y - th
    } else {
        y
    };

    Some((x, ty, tw, th))
}

fn build_plist_region(
    name: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rotated: bool,
    plist_format: i32,
    from_frame: bool,
    page_height: Option<u32>,
) -> Option<AtlasRegion> {
    let ph = page_height.map(|h| h as i32);
    let (x, y, w, h) = normalize_plist_rect(
        x,
        y,
        w,
        h,
        rotated,
        PlistPack {
            format: plist_format,
            from_frame,
        },
        ph,
    )?;
    Some(AtlasRegion {
        name,
        x,
        y,
        width: w,
        height: h,
        rotated,
    })
}

// --- Atlas: raw texture pixels (canvas = PNG, top-left origin) ---

fn atlas_rotate_swaps_dims(value: &str) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "90" | "270" => true,
        "false" => false,
        other => other
            .parse::<i32>()
            .ok()
            .is_some_and(|deg| deg == 90 || deg == 270),
    }
}

fn atlas_has_rotation(value: &str) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "false" => false,
        "true" | "90" | "180" | "270" => true,
        other => other.parse::<i32>().ok().is_some_and(|deg| deg != 0),
    }
}

/// Use atlas xy/size/bounds as-is; only swap packed width/height when rotated 90°/270°.
fn atlas_texture_rect(x: i32, y: i32, w: i32, h: i32, swap_dims: bool) -> Option<(i32, i32, i32, i32)> {
    if w <= 0 || h <= 0 {
        return None;
    }
    if swap_dims {
        Some((x, y, h, w))
    } else {
        Some((x, y, w, h))
    }
}

// --- Plist parsing ---

fn parse_rect_braces(value: &str) -> Option<(i32, i32, i32, i32)> {
    let trimmed = value.trim();
    let normalized = trimmed
        .replace("} , {", "},{")
        .replace("}, {", "},{")
        .replace("} ,{", "},{");

    if let Some(inner) = normalized
        .strip_prefix("{{")
        .and_then(|s| s.strip_suffix("}}"))
    {
        let (pos, size) = inner.split_once("},{")?;
        let (x, y) = pos.split_once(',')?;
        let (w, h) = size.split_once(',')?;
        return Some((
            x.trim().parse().ok()?,
            y.trim().parse().ok()?,
            w.trim().parse().ok()?,
            h.trim().parse().ok()?,
        ));
    }

    let inner = normalized
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() >= 4 {
        return Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
            parts[3].parse().ok()?,
        ));
    }

    None
}

fn parse_size_braces(value: &str) -> Option<(u32, u32)> {
    let inner = value
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))?;
    let (w, h) = inner.split_once(',')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

#[derive(Default)]
struct FrameDraft {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rotated: bool,
    has_frame: bool,
    has_texture_rect: bool,
}

impl FrameDraft {
    fn set_rect(&mut self, x: i32, y: i32, w: i32, h: i32, from_texture_rect: bool) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
        if from_texture_rect {
            self.has_texture_rect = true;
        } else {
            self.has_frame = true;
        }
    }

    fn apply_key(&mut self, key: &str, val: &str) {
        match key {
            "frame" => {
                if let Some(s) = extract_xml_string(val) {
                    if let Some((x, y, w, h)) = parse_rect_braces(&s) {
                        self.set_rect(x, y, w, h, false);
                    }
                }
            }
            "textureRect" => {
                if let Some(s) = extract_xml_string(val) {
                    if let Some((x, y, w, h)) = parse_rect_braces(&s) {
                        self.set_rect(x, y, w, h, true);
                    }
                }
            }
            "x" => {
                if let Some(v) = parse_xml_int(val) {
                    self.x = v;
                    self.has_frame = true;
                }
            }
            "y" => {
                if let Some(v) = parse_xml_int(val) {
                    self.y = v;
                    self.has_frame = true;
                }
            }
            "width" => {
                if let Some(v) = parse_xml_int(val) {
                    self.w = v;
                    self.has_frame = true;
                }
            }
            "height" => {
                if let Some(v) = parse_xml_int(val) {
                    self.h = v;
                    self.has_frame = true;
                }
            }
            "rotated" | "textureRotated" => {
                if let Some(v) = parse_xml_bool(val) {
                    self.rotated = v;
                }
            }
            _ => {}
        }
    }

    fn from_json(frame: &serde_json::Value) -> Self {
        let mut d = Self::default();
        if let Some(rect) = frame.get("frame").and_then(|v| v.as_str()) {
            if let Some((x, y, w, h)) = parse_rect_braces(rect) {
                d.set_rect(x, y, w, h, false);
            }
        }
        if let Some(rect) = frame.get("textureRect").and_then(|v| v.as_str()) {
            if let Some((x, y, w, h)) = parse_rect_braces(rect) {
                d.set_rect(x, y, w, h, true);
            }
        }
        if let Some(v) = frame.get("rotated").or_else(|| frame.get("textureRotated")) {
            d.rotated = v.as_bool().unwrap_or(false);
        }
        d
    }

    fn into_region(
        self,
        name: String,
        plist_format: i32,
        texture_height: Option<u32>,
    ) -> Option<AtlasRegion> {
        let from_frame = self.has_frame && !self.has_texture_rect && plist_format <= 2;
        build_plist_region(
            name,
            self.x,
            self.y,
            self.w,
            self.h,
            self.rotated,
            plist_format,
            from_frame,
            texture_height,
        )
    }
}

fn extract_xml_key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let start = trimmed.find("<key>")? + 5;
    let rest = &trimmed[start..];
    let end = rest.find("</key>")?;
    let key = rest[..end].trim();
    if key.is_empty() { None } else { Some(key.to_string()) }
}

fn extract_xml_string(value: &str) -> Option<String> {
    let start = value.find("<string>")? + "<string>".len();
    let rest = &value[start..];
    let end = rest.find("</string>")?;
    let inner = rest[..end].trim();
    if inner.is_empty() { None } else { Some(inner.to_string()) }
}

fn extract_xml_int(value: &str) -> Option<i32> {
    let start = value.find("<integer>")? + "<integer>".len();
    let rest = &value[start..];
    let end = rest.find("</integer>")?;
    rest[..end].trim().parse().ok()
}

fn parse_xml_int(value: &str) -> Option<i32> {
    extract_xml_int(value).or_else(|| value.trim().parse().ok())
}

fn parse_xml_bool(value: &str) -> Option<bool> {
    let t = value.trim();
    if t.contains("<true") {
        return Some(true);
    }
    if t.contains("<false") {
        return Some(false);
    }
    match t {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn normalize_plist_xml(content: &str) -> String {
    let mut result = String::with_capacity(content.len() + content.len() / 4);
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'>' {
            result.push('>');
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'<' {
                result.push('\n');
            }
            continue;
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn plist_metadata(content: &str) -> (i32, Option<String>, Option<u32>, Option<u32>) {
    let lines: Vec<&str> = content.lines().collect();
    let mut format = 3i32;
    let mut texture_file_name = None;
    let mut texture_width = None;
    let mut texture_height = None;

    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.contains("<key>format</key>") {
            if let Some(v) = extract_xml_int(t) {
                format = v;
            } else {
                for next in lines.iter().skip(i + 1).take(2) {
                    if let Some(v) = extract_xml_int(next.trim()) {
                        format = v;
                        break;
                    }
                }
            }
        }
        if t.contains("<key>textureFileName</key>") || t.contains("<key>realTextureFileName</key>")
        {
            if let Some(v) = extract_xml_string(t) {
                texture_file_name = Some(v);
            } else {
                for next in lines.iter().skip(i + 1).take(3) {
                    if let Some(v) = extract_xml_string(next.trim()) {
                        texture_file_name = Some(v);
                        break;
                    }
                }
            }
        }
        if t.contains("<key>size</key>") {
            let size_str = extract_xml_string(t).or_else(|| {
                lines
                    .iter()
                    .skip(i + 1)
                    .take(3)
                    .find_map(|l| extract_xml_string(l.trim()))
            });
            if let Some(s) = size_str {
                if let Some((w, h)) = parse_size_braces(&s) {
                    texture_width = Some(w);
                    texture_height = Some(h);
                }
            }
        }
    }

    (format, texture_file_name, texture_width, texture_height)
}

fn parse_plist_xml_frames(
    content: &str,
    plist_format: i32,
    texture_height: Option<u32>,
) -> Vec<AtlasRegion> {
    let lines: Vec<&str> = content.lines().collect();
    let Some(start) = lines.iter().position(|line| {
        let t = line.trim();
        t.contains("<key>frames</key>") || t.contains("<key>frames2</key>")
    }) else {
        return Vec::new();
    };

    let mut i = start + 1;
    while i < lines.len() && !lines[i].contains("<dict>") {
        i += 1;
    }
    if i >= lines.len() {
        return Vec::new();
    }
    if lines[i].trim() == "<dict>" {
        i += 1;
    }

    let mut regions = Vec::new();
    while i < lines.len() {
        let t = lines[i].trim();
        if t == "</dict>" || t == "<dict>" {
            i += 1;
            continue;
        }

        let Some(frame_name) = extract_xml_key(t) else {
            i += 1;
            continue;
        };
        if frame_name == "metadata" {
            break;
        }

        if t.contains("<dict>") {
            i += 1;
        } else {
            i += 1;
            while i < lines.len() && !lines[i].contains("<dict>") {
                if lines[i].trim() == "</dict>" {
                    break;
                }
                i += 1;
            }
            if i >= lines.len() || !lines[i].contains("<dict>") {
                continue;
            }
            i += 1;
        }

        let mut draft = FrameDraft::default();
        while i < lines.len() {
            let pline = lines[i].trim();
            if pline == "</dict>" {
                i += 1;
                break;
            }
            if let Some(key) = extract_xml_key(pline) {
                let same_line = pline.contains("</string>")
                    || pline.contains("<integer>")
                    || pline.contains("<true")
                    || pline.contains("<false");
                if same_line {
                    draft.apply_key(&key, pline);
                    i += 1;
                } else {
                    i += 1;
                    if i < lines.len() {
                        draft.apply_key(&key, lines[i].trim());
                    }
                    i += 1;
                }
                continue;
            }
            i += 1;
        }

        if let Some(region) = draft.into_region(frame_name, plist_format, texture_height) {
            regions.push(region);
        }
    }

    regions
}

pub fn parse_cocos_plist(path: &Path) -> Result<AtlasInfo, String> {
    let content = read_config_text(path)?;

    if content.trim_start().starts_with('{') {
        let root: serde_json::Value = serde_json::from_str(&content)
            .map_err(|err| format!("Failed to parse JSON plist \"{}\": {err}", path.display()))?;
        let plist_format = root
            .pointer("/metadata/format")
            .and_then(|v| v.as_i64())
            .unwrap_or(3) as i32;
        let texture_file_name = root
            .pointer("/metadata/textureFileName")
            .or_else(|| root.pointer("/metadata/realTextureFileName"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let size = root
            .pointer("/metadata/size")
            .and_then(|v| v.as_str())
            .and_then(parse_size_braces);
        let texture_width = size.map(|(w, _)| w);
        let texture_height = size.map(|(_, h)| h);
        let frames = root
            .get("frames")
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("JSON plist missing frames: {}", path.display()))?;
        let regions = frames
            .iter()
            .filter_map(|(name, frame)| {
                FrameDraft::from_json(frame).into_region(name.clone(), plist_format, texture_height)
            })
            .collect();
        return Ok(AtlasInfo {
            config_path: path.to_string_lossy().into_owned(),
            config_type: "plist".to_string(),
            texture_file_name,
            texture_width,
            texture_height,
            regions,
        });
    }

    let normalized = normalize_plist_xml(&content);
    let (plist_format, texture_file_name, texture_width, texture_height) =
        plist_metadata(&normalized);
    let regions = parse_plist_xml_frames(&normalized, plist_format, texture_height);

    Ok(AtlasInfo {
        config_path: path.to_string_lossy().into_owned(),
        config_type: "plist".to_string(),
        texture_file_name,
        texture_width,
        texture_height,
        regions,
    })
}

// --- LibGDX / Spine .atlas parsing ---

fn parse_page_size(line: &str) -> Option<(u32, u32)> {
    let rest = line.trim().strip_prefix("size:")?;
    let (pw, ph) = rest.trim().split_once(',')?;
    let pw: u32 = pw.trim().parse().ok()?;
    let ph: u32 = ph.trim().parse().ok()?;
    if pw > 0 && ph > 0 { Some((pw, ph)) } else { None }
}

fn parse_atlas_entry(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.trim().split_once(':')?;
    Some((key.trim(), value.trim()))
}

struct AtlasRegionDraft {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rotated: bool,
    swap_dims: bool,
}

impl AtlasRegionDraft {
    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "xy" => {
                if let Some((px, py)) = value.split_once(',') {
                    self.x = px.trim().parse().unwrap_or(0);
                    self.y = py.trim().parse().unwrap_or(0);
                }
            }
            "size" => {
                if let Some((pw, ph)) = value.split_once(',') {
                    self.w = pw.trim().parse().unwrap_or(0);
                    self.h = ph.trim().parse().unwrap_or(0);
                }
            }
            "bounds" => {
                let p: Vec<&str> = value.split(',').map(str::trim).collect();
                if p.len() >= 4 {
                    self.x = p[0].parse().unwrap_or(0);
                    self.y = p[1].parse().unwrap_or(0);
                    self.w = p[2].parse().unwrap_or(0);
                    self.h = p[3].parse().unwrap_or(0);
                }
            }
            "rotate" => {
                self.rotated = atlas_has_rotation(value);
                self.swap_dims = atlas_rotate_swaps_dims(value);
            }
            _ => {}
        }
    }
}

struct PendingAtlasRegion {
    name: String,
    draft: AtlasRegionDraft,
    y_offset: i32,
}

pub fn parse_libgdx_atlas(path: &Path) -> Result<AtlasInfo, String> {
    let content = read_config_text(path)?;

    let texture_file_name = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.contains(':'))
        .map(str::to_string);

    let mut texture_width = None;
    let mut page_height = 0i32;
    let mut page_texture = texture_file_name.clone().unwrap_or_default();
    let mut y_page_offset = 0i32;
    let mut stacked_height = 0i32;

    for line in content.lines() {
        if let Some((pw, ph)) = parse_page_size(line) {
            texture_width = Some(pw);
            page_height = ph as i32;
            stacked_height = ph as i32;
            break;
        }
    }

    let mut pending: Vec<PendingAtlasRegion> = Vec::new();
    let mut lines = content.lines().peekable();

    // Skip first page header block.
    while lines.peek().is_some_and(|l| l.trim().is_empty()) {
        lines.next();
    }
    if lines
        .peek()
        .is_some_and(|l| !l.trim().is_empty() && !l.trim().contains(':'))
    {
        lines.next();
    }
    while lines.peek().is_some_and(|l| {
        let t = l.trim();
        !t.is_empty() && t.contains(':') && !l.starts_with(' ') && !l.starts_with('\t')
    }) {
        lines.next();
    }
    while lines.peek().is_some_and(|l| l.trim().is_empty()) {
        lines.next();
    }

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((pw, ph)) = parse_page_size(trimmed) {
            texture_width = Some(pw);
            page_height = ph as i32;
            stacked_height = y_page_offset + ph as i32;
            continue;
        }

        if lines
            .peek()
            .is_some_and(|next| next.trim().starts_with("size:"))
        {
            let next_texture = trimmed.to_string();
            let mut pw = 0u32;
            let mut ph = 0u32;
            if let Some(size_line) = lines.next() {
                if let Some((w, h)) = parse_page_size(size_line.trim()) {
                    pw = w;
                    ph = h;
                }
            }
            while let Some(&next_line) = lines.peek() {
                let next = next_line.trim();
                if next.is_empty() {
                    lines.next();
                    break;
                }
                if next.contains(':') && !next_line.starts_with(' ') && !next_line.starts_with('\t')
                {
                    lines.next();
                    continue;
                }
                break;
            }
            if pw > 0 && ph > 0 {
                if next_texture == page_texture && page_height > 0 {
                    y_page_offset += page_height;
                } else {
                    y_page_offset = 0;
                    page_texture = next_texture;
                }
                texture_width = Some(pw);
                page_height = ph as i32;
                stacked_height = y_page_offset + ph as i32;
            }
            continue;
        }

        let name = trimmed.to_string();
        let mut draft = AtlasRegionDraft {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            rotated: false,
            swap_dims: false,
        };

        while let Some(&prop_line) = lines.peek() {
            if prop_line.trim().is_empty()
                || (!prop_line.starts_with(' ') && !prop_line.starts_with('\t'))
            {
                break;
            }
            lines.next();
            if let Some((key, value)) = parse_atlas_entry(prop_line) {
                draft.apply(key, value);
            }
        }

        if draft.w > 0 && draft.h > 0 {
            pending.push(PendingAtlasRegion {
                name,
                draft,
                y_offset: y_page_offset,
            });
        }
    }

    if stacked_height <= 0 {
        stacked_height = page_height;
    }
    let texture_height = if stacked_height > 0 {
        Some(stacked_height as u32)
    } else {
        None
    };

    let regions = pending
        .into_iter()
        .filter_map(|p| {
            let (x, y, w, h) = atlas_texture_rect(
                p.draft.x,
                p.draft.y + p.y_offset,
                p.draft.w,
                p.draft.h,
                p.draft.swap_dims,
            )?;
            Some(AtlasRegion {
                name: p.name,
                x,
                y,
                width: w,
                height: h,
                rotated: p.draft.rotated,
            })
        })
        .collect();

    Ok(AtlasInfo {
        config_path: path.to_string_lossy().into_owned(),
        config_type: "atlas".to_string(),
        texture_file_name,
        texture_width,
        texture_height,
        regions,
    })
}
