import { invoke } from '@tauri-apps/api/core';

const inTauri = '__TAURI_INTERNALS__' in window;

// Outside Tauri (plain browser preview) notes live in localStorage so the UI can be checked without the app.
// shortcut: browser-only fallback for UI checks, never used in the real app.
const fake = {
  all: () => JSON.parse(localStorage.getItem('envelo.fake') || '[]'),
  put(list) { localStorage.setItem('envelo.fake', JSON.stringify(list)); },
};
const api = inTauri
  ? {
      list: () => invoke('list_notes'),
      save: (path, body, created) => invoke('save_note', { path, body, created }),
      trash: (path) => invoke('trash_note', { path }),
    }
  : {
      list: async () => fake.all(),
      async save(path, body, created) {
        const list = fake.all().filter((n) => n.path !== path);
        const note = { path: path ?? `${crypto.randomUUID()}.md`, body, created, modified: new Date().toISOString() };
        fake.put([...list, note]);
        return note;
      },
      async trash(path) { fake.put(fake.all().filter((n) => n.path !== path)); },
    };

export const titleOf = (body) =>
  body.split('\n').map((l) => l.replace(/^#+\s*/, '').trim()).find(Boolean) ?? '';

const plain = (s) => s.replace(/!\[[^\]]*\]\([^)]*\)/g, '').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/<[^>]+>/g, '').replace(/\|/g, ' ').replace(/[*_~`>#\[\]()!-]/g, '').replace(/\s+/g, ' ').trim();
const previewOf = (body) => {
  const lines = body.split('\n').map(plain).filter(Boolean);
  return lines.slice(1).join(' ').slice(0, 140);
};

export const store = $state({ notes: [], currentId: null, query: '', loaded: false, error: null });
let nextId = 1;

const decorate = (n) => ({ ...n, id: nextId++, title: titleOf(n.body), preview: previewOf(n.body), queue: Promise.resolve() });

export async function load() {
  store.notes = (await api.list()).map(decorate);
  store.currentId = sorted(store.notes)[0]?.id ?? null;
  store.loaded = true;
}

/** Picks up notes written elsewhere (quick capture) without disturbing the one being edited. */
export async function refresh() {
  const known = new Set(store.notes.map((n) => n.path));
  const fresh = (await api.list()).filter((n) => !known.has(n.path)).map(decorate);
  if (!fresh.length) return;
  store.notes = [...fresh, ...store.notes];
  if (store.currentId == null) store.currentId = sorted(fresh)[0].id;
}

export const flushAll = () => Promise.all(store.notes.map(flush));

export const sorted = (notes) => [...notes].sort((a, b) => b.modified.localeCompare(a.modified));
export const current = () => store.notes.find((n) => n.id === store.currentId) ?? null;

export function visible() {
  const words = store.query.toLowerCase().split(/\s+/).filter(Boolean);
  const list = sorted(store.notes);
  if (!words.length) return list;
  return list.filter((n) => {
    const hay = (n.title + '\n' + n.body).toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}

const timers = new Map();

/** Called on every edit: updates the list right away, writes the file shortly after typing pauses. */
export function edit(note, body) {
  note.body = body;
  note.title = titleOf(body);
  note.preview = previewOf(body);
  note.modified = new Date().toISOString();
  note.dirty = true;
  clearTimeout(timers.get(note.id));
  timers.set(note.id, setTimeout(() => flush(note), 400));
}

/** Saves are chained per note so a rename never races the next write. */
export function flush(note) {
  clearTimeout(timers.get(note.id));
  timers.delete(note.id);
  if (!note.dirty || (!note.body.trim() && !note.path)) return note.queue;
  note.dirty = false;
  note.queue = note.queue.then(async () => {
    try {
      const saved = await api.save(note.path, note.body, note.created);
      note.path = saved.path;
      note.modified = saved.modified;
    } catch (e) {
      console.error('Could not save note', e);
      store.error = `Couldn't save "${note.title || 'Untitled'}". Your text is still here; it will retry on the next edit.`;
    }
  });
  return note.queue;
}

export function select(id) {
  const prev = current();
  if (prev && prev.id !== id) {
    flush(prev);
    if (!prev.body.trim() && !prev.path) store.notes = store.notes.filter((n) => n !== prev);
  }
  store.currentId = id;
}

export function create(body = '') {
  const now = new Date().toISOString();
  const note = decorate({ path: null, body, created: now, modified: now });
  store.notes = [note, ...store.notes];
  select(note.id);
  if (body) { note.dirty = true; flush(note); }
  return note;
}

export async function remove(note) {
  clearTimeout(timers.get(note.id));
  await note.queue;
  if (note.path) await api.trash(note.path);
  const list = sorted(store.notes);
  const i = list.findIndex((n) => n.id === note.id);
  store.notes = store.notes.filter((n) => n.id !== note.id);
  if (store.currentId === note.id) store.currentId = (list[i + 1] ?? list[i - 1])?.id ?? null;
}

// ---- dates ----
const day = 864e5;
const startOfDay = (d) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();

export function groupOf(iso, now = new Date()) {
  const d = new Date(iso), diff = (startOfDay(now) - startOfDay(d)) / day;
  if (diff <= 0) return 'Today';
  if (diff === 1) return 'Yesterday';
  if (diff < 7) return 'Previous 7 days';
  if (diff < 30) return 'Previous 30 days';
  return d.toLocaleDateString('en-GB', { month: 'long', year: d.getFullYear() === now.getFullYear() ? undefined : 'numeric' });
}

export function timeOf(iso, now = new Date()) {
  const d = new Date(iso), diff = (startOfDay(now) - startOfDay(d)) / day;
  if (diff <= 1) return d.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
  if (diff < 7) return d.toLocaleDateString('en-GB', { weekday: 'short' });
  return d.toLocaleDateString('en-GB', { day: '2-digit', month: '2-digit', year: d.getFullYear() === now.getFullYear() ? undefined : '2-digit' });
}

export function stampOf(note) {
  const f = (iso) => new Date(iso).toLocaleString('en-GB', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
  const c = f(note.created), m = f(note.modified);
  if (c === m) return c;
  const sameDay = new Date(note.created).toDateString() === new Date(note.modified).toDateString();
  return `${c} · edited ${sameDay ? new Date(note.modified).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' }) : m}`;
}
