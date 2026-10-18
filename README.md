# Envelo

A quiet, fast notes app for my laptop.

## Stack

- Tauri 2 (desktop shell, uses the WebView2 already on Windows)
- Svelte (UI, tiny runtime)
- CodeMirror 6 (editor, stays smooth on long notes)
- Notes saved as plain `.md` files in a folder

## Speed goals

- Opens in under a second
- No white flash: the window shows only once it is ready
- Optional: lives in the tray with a global shortcut for instant open

## v1 features

- Create, edit, delete notes, with autosave
- Sidebar list of notes
- Search by title and text
- Markdown: headings, bold/italic, lists, checkboxes, code, with edit/preview
- Folders (real folders on disk)

## Not in v1

Cloud sync, accounts, sharing, attachments.
