use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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

#[tauri::command]
fn notes_root(app: tauri::AppHandle) -> Result<String, String> {
    Ok(root(&app)?.to_string_lossy().into_owned())
}

/// Pasted or dropped pictures arrive as raw bytes (no JSON), with the extension in a header.
#[tauri::command]
fn save_image(app: tauri::AppHandle, request: tauri::ipc::Request) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err("expected image bytes".into()) };
    let ext = request.headers().get("x-ext").and_then(|v| v.to_str().ok()).unwrap_or("png").to_ascii_lowercase();
    if !["png", "jpg", "jpeg", "gif", "webp"].contains(&ext.as_str()) {
        return Err(format!("unsupported image type: {ext}"));
    }
    if bytes.len() > 40 * 1024 * 1024 {
        return Err("image is larger than 40 MB".into());
    }
    let dir = root(&app)?.join("images");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let name = format!("{stamp}.{ext}");
    fs::write(dir.join(&name), bytes).map_err(|e| e.to_string())?;
    Ok(format!("images/{name}"))
}

fn page_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let open = start + lower[start..].find('>')? + 1;
    let close = open + lower[open..].find("</title")?;
    let raw = html[open..close]
        .replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'")
        .replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ");
    let t: String = raw.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(120).collect();
    (!t.is_empty()).then_some(t)
}

/// Looks up a page title for a pasted link. Runs off the UI thread and never blocks typing; offline it just fails.
#[tauri::command]
async fn fetch_title(url: String) -> Option<String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return None;
    }
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(5)).redirects(5).build();
        let res = agent.get(&url).set("User-Agent", "Envelo/0.1 (notes app)").call().ok()?;
        let mut buf = String::new();
        res.into_reader().take(256 * 1024).read_to_string(&mut buf).ok()?;
        page_title(&buf)
    })
    .await
    .ok()
    .flatten()
}

/// Set when Windows starts Envelo at sign-in: it waits in the tray instead of opening a window.
static STARTED_HIDDEN: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn started_hidden() -> bool {
    STARTED_HIDDEN.load(Ordering::Relaxed)
}

/// Called by the main window once it has saved everything.
#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn show_capture(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("capture") {
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("capture-open", ());
    }
}

fn request_quit(app: &tauri::AppHandle) {
    // the main window flushes unsaved edits, then calls `quit`; exit anyway if it can't answer
    let _ = app.emit_to("main", "quit-requested", ());
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(3));
        app.exit(0);
    });
}

fn tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Envelo", true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", "Quick note    Ctrl+Shift+Space", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Envelo", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &capture, &PredefinedMenuItem::separator(app)?, &quit])?;
    TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("Envelo")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show_main(app),
            "capture" => show_capture(app),
            "quit" => request_quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let capture_keys = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
    tauri::Builder::default()
        // a second launch (Start menu, taskbar) just brings the running Envelo forward
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, sc, ev| {
                    if *sc == capture_keys && ev.state() == ShortcutState::Pressed {
                        show_capture(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            STARTED_HIDDEN.store(std::env::args().any(|a| a == "--hidden"), Ordering::Relaxed);
            if let Err(e) = app.global_shortcut().register(capture_keys) {
                eprintln!("Ctrl+Shift+Space is taken by another app: {e}");
            }
            tray(app)?;
            Ok(())
        })
        // closing the window keeps Envelo in the tray so quick capture stays instant
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.label() == "main" {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        })
        // the front end shows the window once the note is painted; this is the safety net if it can't
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && !STARTED_HIDDEN.load(Ordering::Relaxed)
                && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
            {
                let w = webview.window();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    let _ = w.show();
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_notes, save_note, trash_note, notes_root, save_image, fetch_title, started_hidden, quit
        ])
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
    fn reads_page_titles() {
        assert_eq!(page_title("<html><head><TITLE>
 Luke &amp; Co
</TITLE>").as_deref(), Some("Luke & Co"));
        assert_eq!(page_title("<p>no title</p>"), None);
    }

    #[test]
    fn rejects_paths_outside_the_folder() {
        let r = Path::new("C:/notes");
        assert!(inside(r, "../x.md").is_err());
        assert!(inside(r, "C:/Windows/x.md").is_err());
        assert!(inside(r, "Folder/x.md").is_ok());
    }
}
