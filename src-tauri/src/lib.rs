use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::Manager;

#[derive(Serialize)]
struct Note {
    path: String,
    body: String,
    created: String,
    modified: String,
}

/// Notes live as plain .md files in Documents\Envelo.
fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().document_dir().map_err(|e| e.to_string())?.join("Envelo");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Paths come from the web view: only plain relative paths inside the notes folder are allowed.
fn inside(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if rel.is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(format!("invalid note path: {rel}"));
    }
    Ok(root.join(p))
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// Splits the small front-matter block (created/modified) from the note body.
fn split(text: &str) -> (Option<String>, Option<String>, String) {
    let text = text.replace("\r\n", "\n");
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let (mut created, mut modified) = (None, None);
            for line in rest[..end].lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let v = v.trim().to_string();
                    match k.trim() {
                        "created" => created = Some(v),
                        "modified" => modified = Some(v),
                        _ => {}
                    }
                }
            }
            return (created, modified, rest[end + 5..].to_string());
        }
    }
    (None, None, text)
}

fn title_of(body: &str) -> String {
    body.lines()
        .map(|l| l.trim_start_matches('#').trim())
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Windows-safe file name from the note title.
fn file_stem(title: &str) -> String {
    let s: String = title
        .chars()
        .filter(|c| !c.is_control() && !r#"<>:"/\|?*"#.contains(*c))
        .take(80)
        .collect();
    let s = s.trim().trim_end_matches('.').trim().to_string();
    if s.is_empty() { "Untitled".into() } else { s }
}

fn mtime(p: &Path) -> String {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .map(|t| chrono::DateTime::<chrono::Local>::from(t).to_rfc3339())
        .unwrap_or_default()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with('.') {
            continue; // .trash, .locked, .envelo
        }
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

#[tauri::command]
fn list_notes(app: tauri::AppHandle) -> Result<Vec<Note>, String> {
    let root = root(&app)?;
    let mut files = Vec::new();
    walk(&root, &mut files);
    Ok(files
        .iter()
        .filter_map(|p| {
            let text = fs::read_to_string(p).ok()?;
            let (created, modified, body) = split(&text);
            let modified = modified.unwrap_or_else(|| mtime(p));
            Some(Note {
                path: rel(&root, p),
                created: created.unwrap_or_else(|| modified.clone()),
                modified,
                body,
            })
        })
        .collect())
}

/// Writes the note (renaming the file when its title changed) and returns its new relative path.
#[tauri::command]
fn save_note(app: tauri::AppHandle, path: Option<String>, body: String, created: String) -> Result<Note, String> {
    let root = root(&app)?;
    let old = path.as_deref().map(|p| inside(&root, p)).transpose()?;
    let dir = old.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or(root.clone());
    let stem = file_stem(&title_of(&body));

    let target = match &old {
        Some(p) if p.file_stem().is_some_and(|s| s.to_string_lossy() == stem) => p.clone(),
        _ => {
            let mut n = 1;
            loop {
                let name = if n == 1 { format!("{stem}.md") } else { format!("{stem} {n}.md") };
                let candidate = dir.join(name);
                if !candidate.exists() { break candidate; }
                n += 1;
            }
        }
    };

    let modified = chrono::Local::now().to_rfc3339();
    let text = format!("---\ncreated: {created}\nmodified: {modified}\n---\n{body}");
    // write to a temp file first so a crash mid-write never leaves a half-written note
    let tmp = target.with_extension("md.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
    if let Some(old) = old.filter(|o| *o != target) {
        fs::remove_file(old).map_err(|e| e.to_string())?;
    }
    Ok(Note { path: rel(&root, &target), body, created, modified })
}

/// Moves a note into .trash (kept for 30 days, cleanup comes with the Recently deleted view).
#[tauri::command]
fn trash_note(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let root = root(&app)?;
    let src = inside(&root, &path)?;
    let trash = root.join(".trash");
    fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
    let name = format!("{} {}", chrono::Local::now().format("%Y%m%d-%H%M%S"), src.file_name().unwrap_or_default().to_string_lossy());
    fs::rename(&src, trash.join(name)).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // the front end shows the window once the note is painted; this is the safety net if it can't
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let w = webview.window();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    let _ = w.show();
                });
            }
        })
        .invoke_handler(tauri::generate_handler![list_notes, save_note, trash_note])
        .run(tauri::generate_context!())
        .expect("error while running Envelo");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_round_trip() {
        let (c, m, body) = split("---\r\ncreated: 2026-10-18T16:20:00+05:30\r\nmodified: x\r\n---\r\n# Hi\nthere");
        assert_eq!(c.as_deref(), Some("2026-10-18T16:20:00+05:30"));
        assert_eq!(m.as_deref(), Some("x"));
        assert_eq!(body, "# Hi\nthere");
        assert_eq!(split("plain").2, "plain");
    }

    #[test]
    fn titles_and_file_names() {
        assert_eq!(title_of("\n\n## Website ideas\nmore"), "Website ideas");
        assert_eq!(file_stem("a/b: c?."), "ab c");
        assert_eq!(file_stem("   "), "Untitled");
    }

    #[test]
    fn rejects_paths_outside_the_folder() {
        let r = Path::new("C:/notes");
        assert!(inside(r, "../x.md").is_err());
        assert!(inside(r, "C:/Windows/x.md").is_err());
        assert!(inside(r, "Folder/x.md").is_ok());
    }
}
