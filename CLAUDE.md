# Envelo

Personal notes app for Bhavyam's Windows laptop. Full spec: SPEC.md. Product context: PRODUCT.md.
Approved visual design: design/mockup.html (the app's CSS in src/app.css is carried over from it).

## Status

Design approved and building started on 18 Oct 2026. Build in milestones; check each one in the
real app (`npm run tauri dev`) before calling it done. Design changes still get shown to Bhavyam
before they go into the app.

## Layout

- `src/` Svelte front end: `App.svelte` (shell, list), `lib/Editor.svelte` (TipTap),
  `lib/notes.svelte.js` (notes state, autosave, search), `lib/settings.svelte.js`.
- `src-tauri/` Rust: file storage commands in `src/lib.rs` (notes are .md files in
  Documents\Envelo with a small front-matter block).
- In a plain browser (`npm run dev`) notes fall back to localStorage, for UI checks only.

## Commit rules

- Never add `Co-Authored-By: Claude ...` or `Claude-Session: ...` lines to commit messages.
- Never add "Generated with Claude Code" footers to PR descriptions.
- Commit as Bhavyam Misra <misrabhavyam@gmail.com> with normal, concise messages.
