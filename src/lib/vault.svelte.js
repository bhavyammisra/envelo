import { invoke } from '@tauri-apps/api/core';
import { titleOf } from './notes.svelte.js';

const inTauri = '__TAURI_INTERNALS__' in window;
const call = (cmd, args) => (inTauri ? invoke(cmd, args) : Promise.reject(new Error('The locked folder only works in the app')));

// resetting: opened with the security answer, so a new password must be chosen before the notes show
export const vault = $state({ exists: false, unlocked: false, question: null, wait: 0, notes: [], currentId: null, busy: false, resetting: false });

let nextId = 1;
const plain = (s) => s.replace(/<[^>]+>/g, '').replace(/[*_~`>#\[\]()!|-]/g, '').replace(/\s+/g, ' ').trim();
const decorate = (n) => {
  const lines = n.body.split('\n').map(plain).filter(Boolean);
  return { ...n, key: nextId++, title: titleOf(n.body), preview: lines.slice(1).join(' ').slice(0, 140), dirty: false, queue: Promise.resolve() };
};

/** Turns "wrong:3" / "wait:60" from the app into words. */
export function explain(e) {
  const m = String(e).match(/^(wrong|wait):(\d+)/);
  if (!m) return String(e);
  const n = Number(m[2]);
  if (m[1] === 'wait') return `Too many tries. Try again in ${n} ${n === 1 ? 'second' : 'seconds'}.`;
  return `That's not right. ${n} ${n === 1 ? 'try' : 'tries'} left before a short wait.`;
}

export async function status() {
  if (!inTauri) return;
  Object.assign(vault, await invoke('vault_status'));
  if (vault.unlocked && !vault.notes.length) await loadNotes();
}

async function loadNotes() {
  vault.notes = (await call('locked_list')).map(decorate).sort((a, b) => Date.parse(b.modified) - Date.parse(a.modified));
  vault.currentId = vault.notes[0]?.key ?? null;
}

async function busy(fn) {
  vault.busy = true;
  try { return await fn(); } finally { vault.busy = false; }
}

export const setup = (password, question, answer) =>
  busy(async () => { await call('vault_setup', { password, question, answer }); await status(); await loadNotes(); });

export const unlock = (secret, byAnswer = false) =>
  busy(async () => {
    try { await call('vault_unlock', { secret, byAnswer }); } finally { await status(); }
    vault.resetting = byAnswer;
    await loadNotes();
  });

export const setPassword = (password) => busy(async () => { await call('vault_set_password', { password }); vault.resetting = false; });
export const setQuestion = (question, answer) => busy(async () => { await call('vault_set_question', { question, answer }); await status(); });

export async function lock() {
  await Promise.all(vault.notes.map(flush));
  await call('vault_lock');
  vault.resetting = false;
  vault.notes = [];
  vault.currentId = null;
  await status();
}

// ---- locked notes: same autosave rhythm as ordinary notes ----
const timers = new Map();
export const current = () => vault.notes.find((n) => n.key === vault.currentId) ?? null;

export function edit(note, body) {
  note.body = body;
  note.title = titleOf(body);
  note.preview = decorate({ ...note, body }).preview;
  note.modified = new Date().toISOString();
  note.dirty = true;
  clearTimeout(timers.get(note.key));
  timers.set(note.key, setTimeout(() => flush(note), 400));
}

export function flush(note) {
  clearTimeout(timers.get(note.key));
  if (!note.dirty || (!note.body.trim() && !note.id)) return note.queue;
  note.dirty = false;
  note.queue = note.queue.then(async () => {
    const saved = await call('locked_save', { id: note.id || null, body: note.body, created: note.created });
    note.id = saved.id;
    note.modified = saved.modified;
  });
  return note.queue;
}

export function create() {
  const now = new Date().toISOString();
  const note = decorate({ id: '', body: '', created: now, modified: now });
  vault.notes = [note, ...vault.notes];
  vault.currentId = note.key;
  return note;
}

export function select(key) {
  const prev = current();
  if (prev && prev.key !== key) {
    flush(prev);
    if (!prev.body.trim() && !prev.id) vault.notes = vault.notes.filter((n) => n !== prev);
  }
  vault.currentId = key;
}

const drop = (note) => {
  const i = vault.notes.indexOf(note);
  vault.notes = vault.notes.filter((n) => n !== note);
  if (vault.currentId === note.key) vault.currentId = (vault.notes[i] ?? vault.notes[i - 1])?.key ?? null;
};

/** Locked notes are deleted for good (the ordinary trash isn't encrypted). */
export async function remove(note) {
  clearTimeout(timers.get(note.key));
  await note.queue;
  if (note.id) await call('locked_delete', { id: note.id });
  drop(note);
}

/** Back out to the ordinary notes, as a plain file. Returns its path. */
export async function moveOut(note) {
  await flush(note);
  const path = note.id ? await call('unlock_note', { id: note.id }) : null;
  drop(note);
  return path;
}

/** Ordinary note into the locked folder: it's encrypted, then the plain file is removed. */
export async function moveIn(path) {
  const saved = await call('lock_note', { path });
  const note = decorate(saved);
  vault.notes = [note, ...vault.notes];
  return note;
}

// ---- lock again when idle ----
const IDLE_MS = 5 * 60 * 1000;
let idle;
const nudge = () => {
  clearTimeout(idle);
  if (vault.unlocked) idle = setTimeout(() => lock(), IDLE_MS);
};
for (const ev of ['keydown', 'pointerdown', 'pointermove', 'wheel']) addEventListener(ev, nudge, { passive: true });
$effect.root(() => { $effect(() => { if (vault.unlocked) nudge(); else clearTimeout(idle); }); });
