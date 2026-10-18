// shortcut: settings live in the web view's localStorage; move to .envelo/settings.json when the notes folder becomes changeable.
const KEY = 'envelo.settings';
const defaults = { theme: 'system', accent: 'gold', sel: 'liquid' };

let saved = {};
try { saved = JSON.parse(localStorage.getItem(KEY) || '{}'); } catch {}

export const settings = $state({ ...defaults, ...saved });

const dark = matchMedia('(prefers-color-scheme: dark)');
const root = document.documentElement;

function paint() {
  root.dataset.theme = settings.theme === 'dark' || (settings.theme === 'system' && dark.matches) ? 'dark' : 'light';
  root.dataset.accent = settings.accent;
  root.dataset.sel = settings.sel;
}

export function set(key, value) {
  const apply = () => {
    settings[key] = value;
    try { localStorage.setItem(KEY, JSON.stringify(settings)); } catch {}
    paint();
  };
  // the theme change cross-fades as one snapshot instead of repainting every element
  if (key === 'theme' && document.startViewTransition) document.startViewTransition(apply);
  else apply();
}

dark.addEventListener('change', () => settings.theme === 'system' && paint());
// other Envelo windows (quick capture) follow changes made in Settings
addEventListener('storage', (e) => {
  if (e.key !== KEY) return;
  try { Object.assign(settings, JSON.parse(e.newValue || '{}')); } catch {}
  paint();
});
paint();
