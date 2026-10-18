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
- No splash screen: the window is shown only once the last open note is painted, so it opens
  straight onto your note.

## Writing

- First line is the title. Autosave as you type.
- Markdown shortcuts: `#`, `##`, `###` + space for H1 to H3, `-` or `*` for bullets, `1.` for
  numbers, `[]` for checkbox, `>` for quote, `---` for divider.
- `/` menu: Heading 1, 2, 3, Text, Bullets, Numbers, Checklist, Quote, Divider, Table, Image.
- Selecting text shows a small format bar: H1 H2 H3, bold, italic, underline, strikethrough,
  highlight (a few soft colours), link.
- Tables: insert via `/table` (3 by 3 with a header row). While the cursor is in a table, a bar
  above it offers Add row, Remove row, Add column, Remove column, Delete table. Saved as
  Markdown tables.
- Images: paste (Ctrl+V), drag in, or `/` Picture to pick files (PNG, JPG, GIF, WebP, up to 40 MB).
  Saved to `images/` in the notes folder and written as `![](images/name.png)` on its own line.
  Full width with soft corners; click to view larger, Esc or click to close. Loaded lazily.
- Every note always ends with an empty line so you can type after a picture, table or card.
- Links: a URL pasted on an empty line becomes a link card (site initial, page title, site name).
  The title is fetched in the background when online (5 s timeout) and never blocks typing; cards
  saved offline get their title the next time the note is opened online. Saved in the file as
  `<div data-link-card="url">Title</div>`. Clicking a card, or Ctrl+clicking a text link, opens it
  in the default browser. URLs pasted inside text stay normal links.
- Note background per note: plain, warm paper, lined, grid, dotted.
- Smart typography as you type: straight quotes become curly quotes, `->` becomes an arrow,
  `...` becomes an ellipsis, `1/2` becomes a fraction. Undo (Ctrl+Z) right after reverts it.
  Off inside code. Numbers use tabular figures so they line up.
- Focus mode (Ctrl+.): hides the sidebar and notes list and widens the page; Esc or Ctrl+.
  leaves it.

## Search

Ctrl+K focuses search. Matches keywords in titles and text, results as you type, matched words
highlighted. Index kept in memory, built at start-up from the files.

## Quick capture

- Ctrl+Shift+Space from anywhere in Windows opens a small box in the middle of the screen, over
  the current app (a separate always-on-top window).
- Enter saves it as a new note, Shift+Enter adds a line, Esc or clicking away closes it. A
  dismissed draft is kept for next time.
- Each line becomes its own paragraph; a line that is only a link becomes a link card (its title
  is fetched when the note is opened).
- On Enter the box folds up and away (about 0.35 s) and "Saved to Envelo" shows briefly. The
  main window's list picks up the new note immediately.

## Tray and startup

- Closing the main window hides it; Envelo keeps running in the tray so quick capture is
  instant. Unsaved edits are written first.
- Tray icon: left-click opens Envelo. Menu: Open Envelo, Quick note, Quit Envelo (saves pending
  edits, then exits).
- Starting Envelo again (Start menu, taskbar) brings the running window forward; there is only
  ever one copy running.
- Settings, "Start with Windows" (off by default): launches at sign-in straight into the tray,
  without opening a window.

## Organising

- Pin: the pin button on a note keeps it at the top of All notes under "Pinned", and in the Pinned
  view. Saved as `pinned: true` in the note's front matter; pinning doesn't change the note's date.
- Folders: real folders inside the notes folder (one level). Sidebar: "New" creates one
  (type a name, Enter); double-click renames; the x (then Remove) removes it and moves its notes
  back to All notes, so nothing is lost. Folder names can't be "images" or start with a dot.
- Moving: the folder button on a note opens a menu: No folder, each folder, New folder.
- A note made while a folder is open is created in that folder.
- Delete moves a note to Recently deleted (`.trash`), remembering its folder.
- Recently deleted: lists deleted notes newest first, read-only, with when it was deleted and how
  many days are left. Restore puts it back in its folder (recreating the folder if needed).
  "Delete now" then "Delete forever" removes it for good. Notes older than 30 days are removed
  when Envelo starts.

## Locked folder

- First open: create a password (6+ characters, typed twice), pick or write a security
  question, enter the answer.
- One random 32-byte key encrypts locked notes (AES-256-GCM). The key is stored twice in
  `.locked/vault.json`, wrapped by keys derived from the password and from the answer
  (Argon2id, 64 MiB, 3 passes). Either unlocks. Each locked note is `.locked/<random>.note`,
  so titles are hidden too. While unlocked the key exists only in memory.
- Answers ignore capitals and extra spaces.
- Forgot password: answer the question, then a new password must be set before the notes show.
- 5 wrong tries in a row: a 1 minute wait (kept across restarts).
- Locks again after 5 minutes without use, with "Lock now", when the window is closed to the
  tray, and on quit.
- Moving in: "Locked folder" in a note's move menu encrypts it, then removes the plain file.
  Moving out: the button on a locked note writes it back as an ordinary note.
- Deleting a locked note is permanent (two clicks), because the ordinary trash isn't encrypted.
- Pictures can't be added to locked notes (they're stored as ordinary files).
- Settings: change the password or the security question while unlocked.
- Forgetting both password and answer means those notes cannot be recovered.
- Known limit: a note moved into Locked may still exist in older OneDrive version history or in
  deleted-file recovery tools, because it was a plain file before. Notes written in Locked from
  the start never touch the disk unencrypted.

## Look

- Light and dark mode, following Windows with a manual override (System, Light, Dark).
- Light: warm paper throughout.
- Dark: Apple Notes colours. Grey sidebar (#1E1E1E), darker notes list (#161616), pure black
  note, white text, grey secondary text. The selected note gets the normal soft highlight (not
  filled with the accent); sidebar icons take the accent colour.
- Accent colour chosen in Settings: Gold (default, like Apple Notes), Ink blue, Terracotta or
  Moss. Each has a light and a dark variant.
- Soft rounded rectangles; no pill buttons, no emoji, no decorative animation.
- Contour lines style: a faint topographic contour map behind its tile in the list (one
  small static SVG, tinted with the text colour, faded at the left edge). Each note shows a
  different patch of the map. It settles in once (about 0.7 s) when the note is selected, then
  stays still. With Windows "reduce animations" on, it appears without motion.
- Default selected note style, Liquid glass (in the spirit of Apple's Liquid Glass): a clear,
  untinted glass lens floats over the selected note. Its rounded edges refract the text beneath
  (SVG displacement map, `design/lens-map.png`), with a bright rim highlight and a soft shadow.
  Clicking another note glides the lens there with a slight springy settle (about 0.55 s).
  Only this one small element uses a backdrop filter.
- Settings, "Selected note style": Liquid glass (default), Contour lines, or None (just the
  plain soft highlight, no effect).
- Theme switch cross-fades in about 0.2 s (one GPU snapshot, no per-element transitions).
- App icon: a sealed envelope on warm paper with a folded top-right corner (see mockup).

## Settings

- Appearance: theme, selected note style (Liquid glass, Contour lines or None), accent colour, default
  background for new notes.
- Notes: notes folder location (shown), quick capture shortcut (shown; changing it is not in v1),
  start with Windows.
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
| Ctrl+. | Focus mode |
| Delete (in list) | Move to Recently deleted |

## Performance budget

- Cold start to usable window: under 1 second on this laptop.
- Typing latency: no visible delay in a 10,000 word note.
- List scrolling smooth with 5,000 notes (only visible rows drawn).
- Search results within 50 ms of a keystroke for 5,000 notes.

## Not in v1

Sync, accounts, AI, a Tasks tab, widgets, templates, photo backgrounds, reminders.
