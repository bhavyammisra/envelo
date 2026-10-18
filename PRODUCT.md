# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Rendered in WebView2 inside a Tauri 2 desktop app for Windows. The design language is the app's
own, not native WinUI.)

## Stack

Tauri 2 + Svelte + TipTap (ProseMirror) editor; thin Rust layer for files, encryption and the
global shortcut. Notes stored as plain `.md` files with an `images` folder. Fully offline.
Chosen by Claude with Bhavyam's agreement (he explicitly ruled out C#/pure-Rust native).

## Users

One user: Bhavyam, on his own Windows laptop. Personal use only, never distributed.
Job: catch any thought, link, website he likes, or screenshot the moment it appears, and find
it again later by keyword.

## Product Purpose

Envelo is one calm place to drop anything: a passing thought, a link, a screenshot, or a proper
formatted note. Success: it opens instantly, never lags, and reading a note feels pleasant.

## Positioning

A desktop answer to Nothing's Essential Space and Apple Notes: a global quick-capture box from
anywhere in Windows, plain files on disk with no account or database, and a locked folder that
is truly encrypted but still recoverable via a security question.

## Operating Context

Used alongside everything else on the laptop. Quick capture (Ctrl+Shift+Space) pops over
whatever app is in front. Longer writing happens in the main window. Bhavyam currently uses a
phone notes app (screenshots shared in planning) and Apple Notes as references.

## Capabilities and Constraints

- Notes list on the left grouped by date (Today, Yesterday, Previous 30 days, months), with
  title, first line and date/time. List view and card view.
- Search at the top by keywords in title and body, instant.
- Editor: H1/H2/H3, bold, italic, underline, strikethrough, highlight, bullets, numbered lists,
  checkboxes, quotes, dividers, tables. Markdown shortcuts plus a `/` menu; a small format bar
  appears on text selection. First line is the title. Autosave.
- Paste or drag images and screenshots. Paste links; page titles fetched in the background when
  online, never blocking.
- Optional folders (off until used). Sidebar: All notes, folders, Pinned, Locked, Recently deleted.
- Pin notes. Trash keeps deleted notes 30 days.
- Locked folder: password set on first use plus a security question. Either unlocks. Notes
  encrypted on disk. Re-locks on close or idle. 5 wrong tries triggers a 1 minute wait.
- Note backgrounds: plain, warm paper, lined, grid, dotted. No photo backgrounds.
- Light and dark mode (follow Windows, manual override).
- Shortcuts: Ctrl+N new note, Ctrl+K search, Ctrl+Shift+Space quick capture.
- Hard constraint: must open in under a second and never feel laggy.
- Out of scope: sync, accounts, AI, Tasks tab, widgets, feedback/community links, templates.

## Brand Commitments

- Name: Envelo. Vibe: warm, minimal, paper and nature, premium not ordinary.
- Fonts (chosen by Bhavyam): Newsreader for titles/headings, Instrument Sans for text. Bundled.
- Accent: muted terracotta (Claude's pick, pending Bhavyam's approval).
- Bans from Bhavyam's standing rules: no purple gradients, no pill-shaped buttons, no emoji
  icons, no em dashes in copy, no cursor or gimmicky scroll animations, no AI slop imagery or copy.

## Evidence on Hand

Reference screenshots of Bhavyam's phone notes app (shared in chat, not in repo). No other assets.

## Product Principles

1. Speed is a feature: nothing ships that delays opening or typing.
2. Capture first, organise later: dropping a thought takes one shortcut and Enter.
3. Quiet by default: controls appear when needed and stay out of the writing.
4. Your files, your laptop: plain files, offline, nothing locked into the app except what you lock.

## Accessibility & Inclusion

Readability is the stated priority: comfortable type size and contrast in both themes, full
keyboard use.
