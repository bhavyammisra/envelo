import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

const inTauri = '__TAURI_INTERNALS__' in window;
let root = null;

export async function initMedia() {
  if (inTauri) root = await invoke('notes_root');
}

/** Notes store pictures as `images/x.png`; the web view needs an asset URL for the real file. */
export function resolveSrc(src) {
  if (!src || /^(https?:|data:|blob:|asset:)/.test(src) || !inTauri || !root) return src;
  return convertFileSrc(`${root}\\${src.replaceAll('/', '\\')}`);
}

const extOf = (file) => ({ 'image/png': 'png', 'image/jpeg': 'jpg', 'image/gif': 'gif', 'image/webp': 'webp' })[file.type];
export const isPicture = (file) => !!extOf(file);

/** Saves a pasted, dropped or picked picture into the notes folder and returns its relative path. */
export async function savePicture(file) {
  if (!inTauri) return URL.createObjectURL(file); // shortcut: browser preview keeps pictures for the session only
  const bytes = new Uint8Array(await file.arrayBuffer());
  return invoke('save_image', bytes, { headers: { 'x-ext': extOf(file) } });
}

export const fetchTitle = (url) => (inTauri ? invoke('fetch_title', { url }).catch(() => null) : Promise.resolve(null));

export const openLink = (url) => (inTauri ? openUrl(url) : window.open(url, '_blank', 'noopener'));

export const hostOf = (url) => {
  try { return new URL(url).hostname.replace(/^www\./, ''); } catch { return url; }
};
