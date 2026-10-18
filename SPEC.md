# Envelo spec (v1)

The full plan agreed in planning, 18 Oct 2026. Product context lives in PRODUCT.md; visual
reference is `design/mockup.html`. Nothing here is built yet.

## Principles

1. Opens in under a second, never lags while typing or scrolling.
2. Capture first, organise later.
3. Quiet by default: controls appear when needed.
4. Plain files, fully offline. No account, no database, no sync.

## Stack

- Tauri 2 desktop shell on WebView2 (already on Windows).
- Svelte for the UI.
- TipTap (ProseMirror) editor, saving to Markdown.
- Thin Rust layer: file read/write and watching, search index, encryption, global shortcut, tray.
- Fonts bundled: Newsreader (titles, headings), Instrument Sans (UI and body text).

## Storage

```
<notes folder>/            chosen on first run, default Documents\Envelo
  Website ideas.md
  <Folder name>/...        optional folders are real folders
  images/                  pasted pictures, referenced from notes
  .locked/                 encrypted locked notes
  .trash/                  deleted notes, kept 30 days
  .envelo/                 settings, pins, lock metadata
```

Each note keeps created and edited times in a small front-matter block.

## Layout

- Custom title bar with Windows minimise, maximise and close.
- Sidebar (collapsible): All notes, Pinned, Folders (shown once one exists, with New folder),
  Locked, Recently deleted, Settings at the bottom.
- Notes list: search on top, then notes grouped Today, Yesterday, Previous 7 days, Previous 30
  days, then by month. Each row: title, date or time, first line of text. List view or card view.
- Editor: created and edited date at top, then the note. Comfortable reading width, centred.
- Window remembers its size, position and last open note.

## Writing

- First line is the title. Autosave as you type.
- Markdown shortcuts: `#`, `##`, `###` + space for H1 to H3, `-` or `*` for bullets, `1.` for
  numbers, `[]` for checkbox, `>` for quote, `---` for divider.
- `/` menu: Heading 1, 2, 3, Text, Bullets, Numbers, Checklist, Quote, Divider, Table, Image.
- Selecting text shows a small format bar: H1 H2 H3, bold, italic, underline, strikethrough,
  highlight (a few soft colours), link.
- Tables: insert via `/table`; add or remove rows and columns from a small menu on the table.
- Images: paste (Ctrl+V) or drag in. Full width with soft corners; click to view larger.
  Images load only when scrolled into view.
- Links: pasted URLs show as a neat line with the site name. Page title fetched in the
  background when online; never blocks typing.
- Note background per note: plain, warm paper, lined, grid, dotted.

## Search

Ctrl+K focuses search. Matches keywords in titles and text, results as you type, matched words
highlighted. Index kept in memory, built at start-up from the files.

## Quick capture

Ctrl+Shift+Space from anywhere in Windows opens a small box over the current app. Type or paste
a thought or link, Enter saves it as a new note, Shift+Enter adds a line, Esc closes. Envelo
keeps running in the tray so this is instant.

## Organising

- Pin notes to the top.
- Optional folders: create, rename, delete; move notes into them. Off until first used.
- Delete moves to Recently deleted; restore any time within 30 days, then removed for good.

## Locked folder

- First open: create a password, pick or write a security question, enter the answer.
- One random key encrypts locked notes (AES-256-GCM). The key is stored twice, wrapped by a key
  derived from the password and by one derived from the answer (Argon2id). Either unlocks.
- Answers ignore case and extra spaces.
- Forgot password: answer the question, then set a new password.
- 5 wrong attempts: wait 1 minute.
- Re-locks when the app closes or after 5 idle minutes.
- Change password or question in settings after unlocking.
- Forgetting both password and answer means those notes cannot be recovered.

## Look

- Light and dark mode, following Windows with a manual override (System, Light, Dark).
- Light: warm paper throughout.
- Dark: Apple Notes colours. Grey sidebar (#1E1E1E), darker notes list (#161616), pure black
  note, white text, grey secondary text. The selected note gets the normal soft highlight (not
  filled with the accent); sidebar icons take the accent colour.
- Accent colour chosen in Settings: Gold (default, like Apple Notes), Ink blue, Terracotta or
  Moss. Each has a light and a dark variant.
- Soft rounded rectangles; no pill buttons, no emoji, no decorative animation.
- Selected note signature: the note's own words, set tiny, italic and very faint in lines behind
  its tile in the list. They slide in once (about half a second) when the note is selected, then
  stay still. Off when Windows "reduce animations" is on (lines shown without motion).

## Settings

- Appearance: theme, accent colour, default background for new notes.
- Notes: notes folder location, quick capture shortcut, start with Windows (tray).
- Locked folder: change password and security question (after unlocking), lock after idle time.

## Shortcuts

| Keys | Action |
|---|---|
| Ctrl+N | New note |
| Ctrl+K | Search |
| Ctrl+Shift+Space | Quick capture (global) |
| Ctrl+B / I / U | Bold / italic / underline |
| Ctrl+1 / 2 / 3 | Heading 1 / 2 / 3 |
| Ctrl+0 | Body text |
| Ctrl+\\ | Toggle sidebar |
| Delete (in list) | Move to Recently deleted |

## Performance budget

- Cold start to usable window: under 1 second on this laptop.
- Typing latency: no visible delay in a 10,000 word note.
- List scrolling smooth with 5,000 notes (only visible rows drawn).
- Search results within 50 ms of a keystroke for 5,000 notes.

## Not in v1

Sync, accounts, AI, a Tasks tab, widgets, templates, photo backgrounds, reminders.
