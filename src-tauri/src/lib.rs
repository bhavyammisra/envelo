mod vault;

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
    pinned: bool,
}

#[derive(Serialize)]
struct Trashed {
    name: String,
    body: String,
    deleted: String,
    folder: Option<String>,
}

/// The small front-matter block at the top of each note file.
#[derive(Default, Debug, PartialEq)]
struct Front {
    created: Option<String>,
    modified: Option<String>,
    pinned: bool,
    folder: Option<String>, // only in .trash: where the note came from
}

/// Notes live as plain .md files in Documents\Envelo.
fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // dev builds can point at a scratch folder so testing never touches real notes
    let test_dir = cfg!(debug_assertions).then(|| std::env::var_os("ENVELO_NOTES_DIR")).flatten();
    let dir = match test_dir {
        Some(d) => PathBuf::from(d),
        None => app.path().document_dir().map_err(|e| e.to_string())?.join("Envelo"),
    };
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

fn split(text: &str) -> (Front, String) {
    let text = text.replace("\r\n", "\n");
    let mut f = Front::default();
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            for line in rest[..end].lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let v = v.trim().to_string();
                    match k.trim() {
                        "created" => f.created = Some(v),
                        "modified" => f.modified = Some(v),
                        "pinned" => f.pinned = v == "true",
                        "folder" => f.folder = Some(v).filter(|v| !v.is_empty()),
                        _ => {}
                    }
                }
            }
            return (f, rest[end + 5..].to_string());
        }
    }
    (f, text)
}

fn join(f: &Front, body: &str) -> String {
    let mut head = format!(
        "---\ncreated: {}\nmodified: {}\n",
        f.created.as_deref().unwrap_or(""),
        f.modified.as_deref().unwrap_or("")
    );
    if f.pinned {
        head.push_str("pinned: true\n");
    }
    if let Some(folder) = &f.folder {
        head.push_str(&format!("folder: {folder}\n"));
    }
    format!("{head}---\n{body}")
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

/// Folder names: one level, Windows-safe, and not one of Envelo's own folders.
fn folder_name(name: &str) -> Result<String, String> {
    let n = file_stem(name);
    if name.trim().is_empty() || n.starts_with('.') || n.eq_ignore_ascii_case("images") {
        return Err(format!("\"{}\" can't be used as a folder name", name.trim()));
    }
    Ok(n)
}

fn unique(dir: &Path, stem: &str) -> PathBuf {
    (1..)
        .map(|n| dir.join(if n == 1 { format!("{stem}.md") } else { format!("{stem} {n}.md") }))
        .find(|p| !p.exists())
        .expect("a free file name")
}

/// Writes to a temp file first so a crash mid-write never leaves a half-written note.
fn write_atomic(target: &Path, text: &str) -> Result<(), String> {
    let tmp = target.with_extension("md.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, target).map_err(|e| e.to_string())
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
            let (f, body) = split(&fs::read_to_string(p).ok()?);
            let modified = f.modified.filter(|m| !m.is_empty()).unwrap_or_else(|| mtime(p));
            Some(Note {
                path: rel(&root, p),
                created: f.created.filter(|c| !c.is_empty()).unwrap_or_else(|| modified.clone()),
                modified,
                body,
                pinned: f.pinned,
            })
        })
        .collect())
}

/// Writes the note (renaming the file when its title changed) and returns it with its new path.
/// `folder` places a brand-new note; `modified` keeps the old time for changes that aren't edits (pinning).
#[tauri::command]
fn save_note(
    app: tauri::AppHandle,
    path: Option<String>,
    body: String,
    created: String,
    pinned: Option<bool>,
    folder: Option<String>,
    modified: Option<String>,
) -> Result<Note, String> {
    let root = root(&app)?;
    let old = path.as_deref().map(|p| inside(&root, p)).transpose()?;
    let dir = match (&old, folder.as_deref().filter(|f| !f.is_empty())) {
        (Some(p), _) => p.parent().map(Path::to_path_buf).unwrap_or(root.clone()),
        (None, Some(f)) => root.join(folder_name(f)?),
        (None, None) => root.clone(),
    };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stem = file_stem(&title_of(&body));
    let target = match &old {
        Some(p) if p.file_stem().is_some_and(|s| s.to_string_lossy() == stem) => p.clone(),
        _ => unique(&dir, &stem),
    };

    let front = Front {
        created: Some(created.clone()),
        modified: Some(modified.unwrap_or_else(|| chrono::Local::now().to_rfc3339())),
        pinned: pinned.unwrap_or(false),
        folder: None,
    };
    write_atomic(&target, &join(&front, &body))?;
    if let Some(old) = old.filter(|o| *o != target) {
        fs::remove_file(old).map_err(|e| e.to_string())?;
    }
    Ok(Note {
        path: rel(&root, &target),
        body,
        created,
        modified: front.modified.unwrap_or_default(),
        pinned: front.pinned,
    })
}

/// Moves a note into .trash, remembering its folder so Restore can put it back.
#[tauri::command]
fn trash_note(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let root = root(&app)?;
    let src = inside(&root, &path)?;
    let trash = root.join(".trash");
    fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
    let (mut front, body) = split(&fs::read_to_string(&src).map_err(|e| e.to_string())?);
    front.folder = src.parent().filter(|p| *p != root).map(|p| rel(&root, p));
    let name = format!(
        "{} {}",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        src.file_name().unwrap_or_default().to_string_lossy()
    );
    write_atomic(&trash.join(name), &join(&front, &body))?;
    fs::remove_file(&src).map_err(|e| e.to_string())
}

/// Trash file names start with the time they were deleted: "20261019-000927 Title.md".
fn deleted_at(name: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(name.get(..15)?, "%Y%m%d-%H%M%S").ok()
}

fn trash_file(root: &Path, name: &str) -> Result<PathBuf, String> {
    if name.contains(['/', '\\']) || name.starts_with('.') {
        return Err(format!("invalid trash entry: {name}"));
    }
    inside(&root.join(".trash"), name)
}

#[tauri::command]
fn list_trash(app: tauri::AppHandle) -> Result<Vec<Trashed>, String> {
    let dir = root(&app)?.join(".trash");
    let Ok(entries) = fs::read_dir(&dir) else { return Ok(vec![]) };
    Ok(entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".md") {
                return None;
            }
            let deleted = deleted_at(&name)?.and_local_timezone(chrono::Local).single()?.to_rfc3339();
            let (front, body) = split(&fs::read_to_string(e.path()).ok()?);
            Some(Trashed { name, body, deleted, folder: front.folder })
        })
        .collect())
}

/// Puts a deleted note back where it was (recreating its folder if needed) and returns its path.
#[tauri::command]
fn restore_note(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let root = root(&app)?;
    let src = trash_file(&root, &name)?;
    let (mut front, body) = split(&fs::read_to_string(&src).map_err(|e| e.to_string())?);
    let dir = match front.folder.take() {
        Some(f) => inside(&root, &f)?,
        None => root.clone(),
    };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let original = name.get(16..).unwrap_or(&name).trim_end_matches(".md");
    let target = unique(&dir, &file_stem(original));
    write_atomic(&target, &join(&front, &body))?;
    fs::remove_file(&src).map_err(|e| e.to_string())?;
    Ok(rel(&root, &target))
}

#[tauri::command]
fn delete_forever(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let root = root(&app)?;
    fs::remove_file(trash_file(&root, &name)?).map_err(|e| e.to_string())
}

/// Notes stay in Recently deleted for 30 days, then they're removed for good.
fn purge_trash(root: &Path) {
    let Ok(entries) = fs::read_dir(root.join(".trash")) else { return };
    let cutoff = chrono::Local::now().naive_local() - chrono::Duration::days(30);
    for e in entries.flatten() {
        if deleted_at(&e.file_name().to_string_lossy()).is_some_and(|t| t < cutoff) {
            let _ = fs::remove_file(e.path());
        }
    }
}

#[tauri::command]
fn list_folders(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let root = root(&app)?;
    let mut out: Vec<String> = fs::read_dir(&root)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.') && !n.eq_ignore_ascii_case("images"))
        .collect();
    out.sort_by_key(|n| n.to_lowercase());
    Ok(out)
}

#[tauri::command]
fn create_folder(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let name = folder_name(&name)?;
    let dir = root(&app)?.join(&name);
    if dir.exists() {
        return Err(format!("A folder called \"{name}\" already exists"));
    }
    fs::create_dir(&dir).map_err(|e| e.to_string())?;
    Ok(name)
}

#[tauri::command]
fn rename_folder(app: tauri::AppHandle, from: String, to: String) -> Result<String, String> {
    let root = root(&app)?;
    let to = folder_name(&to)?;
    let src = inside(&root, &folder_name(&from)?)?;
    let dst = root.join(&to);
    if dst.exists() {
        return Err(format!("A folder called \"{to}\" already exists"));
    }
    fs::rename(src, dst).map_err(|e| e.to_string())?;
    Ok(to)
}

/// Removes a folder; its notes move back to All notes rather than being deleted.
#[tauri::command]
fn delete_folder(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let root = root(&app)?;
    let dir = inside(&root, &folder_name(&name)?)?;
    let mut notes = Vec::new();
    walk(&dir, &mut notes);
    for p in notes {
        let stem = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        fs::rename(&p, unique(&root, &stem)).map_err(|e| e.to_string())?;
    }
    let _ = fs::remove_dir(&dir); // only goes if nothing else (non-note files) is left inside
    Ok(())
}

/// Moves a note into a folder (or back to the top level with `None`) and returns its new path.
#[tauri::command]
fn move_note(app: tauri::AppHandle, path: String, folder: Option<String>) -> Result<String, String> {
    let root = root(&app)?;
    let src = inside(&root, &path)?;
    let dir = match folder.as_deref().filter(|f| !f.is_empty()) {
        Some(f) => root.join(folder_name(f)?),
        None => root.clone(),
    };
    if src.parent() == Some(dir.as_path()) {
        return Ok(path);
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let target = unique(&dir, &src.file_stem().unwrap_or_default().to_string_lossy());
    fs::rename(&src, &target).map_err(|e| e.to_string())?;
    Ok(rel(&root, &target))
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

// ---- locked folder (see vault.rs) ----

/// Runs vault work that may hash a password off the UI thread.
async fn blocking<T: Send + 'static>(
    app: tauri::AppHandle,
    f: impl FnOnce(&Path, &vault::Vault) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = root(&app)?;
        f(&root, &app.state::<vault::Vault>())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn vault_status(app: tauri::AppHandle, state: tauri::State<vault::Vault>) -> Result<vault::Status, String> {
    vault::status(&root(&app)?, &state)
}

#[tauri::command]
async fn vault_setup(app: tauri::AppHandle, password: String, question: String, answer: String) -> Result<(), String> {
    blocking(app, move |r, v| vault::setup(r, v, &password, &question, &answer)).await
}

#[tauri::command]
async fn vault_unlock(app: tauri::AppHandle, secret: String, by_answer: bool) -> Result<(), String> {
    blocking(app, move |r, v| vault::unlock(r, v, &secret, by_answer)).await
}

#[tauri::command]
fn vault_lock(state: tauri::State<vault::Vault>) {
    vault::lock(&state);
}

#[tauri::command]
async fn vault_set_password(app: tauri::AppHandle, password: String) -> Result<(), String> {
    blocking(app, move |r, v| vault::set_password(r, v, &password)).await
}

#[tauri::command]
async fn vault_set_question(app: tauri::AppHandle, question: String, answer: String) -> Result<(), String> {
    blocking(app, move |r, v| vault::set_question(r, v, &question, &answer)).await
}

#[tauri::command]
fn locked_list(app: tauri::AppHandle, state: tauri::State<vault::Vault>) -> Result<Vec<vault::LockedNote>, String> {
    vault::list(&root(&app)?, &state)
}

#[tauri::command]
fn locked_save(app: tauri::AppHandle, state: tauri::State<vault::Vault>, id: Option<String>, body: String, created: String) -> Result<vault::LockedNote, String> {
    vault::save(&root(&app)?, &state, id, body, created)
}

#[tauri::command]
fn locked_delete(app: tauri::AppHandle, state: tauri::State<vault::Vault>, id: String) -> Result<(), String> {
    vault::delete(&root(&app)?, &state, &id)
}

/// Moves an ordinary note into the locked folder: encrypted first, then the plain file is removed.
#[tauri::command]
fn lock_note(app: tauri::AppHandle, state: tauri::State<vault::Vault>, path: String) -> Result<vault::LockedNote, String> {
    let root = root(&app)?;
    let src = inside(&root, &path)?;
    let (front, body) = split(&fs::read_to_string(&src).map_err(|e| e.to_string())?);
    let created = front.created.unwrap_or_else(|| chrono::Local::now().to_rfc3339());
    let note = vault::save(&root, &state, None, body, created)?;
    fs::remove_file(&src).map_err(|e| e.to_string())?;
    Ok(note)
}

/// Moves a locked note back out as an ordinary note in the top level; returns its path.
#[tauri::command]
fn unlock_note(app: tauri::AppHandle, state: tauri::State<vault::Vault>, id: String) -> Result<String, String> {
    let root = root(&app)?;
    let note = vault::read(&root, &state, &id)?;
    let target = unique(&root, &file_stem(&title_of(&note.body)));
    let front = Front { created: Some(note.created), modified: Some(note.modified), pinned: false, folder: None };
    write_atomic(&target, &join(&front, &note.body))?;
    vault::delete(&root, &state, &id)?;
    Ok(rel(&root, &target))
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
        .manage(vault::Vault::default())
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
            if let Ok(r) = root(app.handle()) {
                purge_trash(&r);
            }
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
            list_notes, save_note, trash_note, notes_root, save_image, fetch_title, started_hidden, quit,
            list_trash, restore_note, delete_forever, list_folders, create_folder, rename_folder, delete_folder, move_note,
            vault_status, vault_setup, vault_unlock, vault_lock, vault_set_password, vault_set_question,
            locked_list, locked_save, locked_delete, lock_note, unlock_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running Envelo");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_round_trip() {
        let (f, body) = split("---\r\ncreated: 2026-10-18T16:20:00+05:30\r\nmodified: x\r\npinned: true\r\n---\r\n# Hi\nthere");
        assert_eq!(f.created.as_deref(), Some("2026-10-18T16:20:00+05:30"));
        assert_eq!(f.modified.as_deref(), Some("x"));
        assert!(f.pinned);
        assert_eq!(body, "# Hi\nthere");
        assert_eq!(split("plain").1, "plain");
        let again = split(&join(&f, &body));
        assert_eq!(again.0, f);
        assert_eq!(again.1, body);
    }

    #[test]
    fn folder_names_and_trash_names() {
        assert_eq!(folder_name(" College ").unwrap(), "College");
        assert!(folder_name("images").is_err());
        assert!(folder_name(".trash").is_err());
        assert!(folder_name("  ").is_err());
        assert!(deleted_at("20261019-000927 Note.md").is_some());
        assert!(deleted_at("Note.md").is_none());
        assert!(trash_file(Path::new("C:/n"), "../x.md").is_err());
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
