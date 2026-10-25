<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen, emitTo } from '@tauri-apps/api/event';
  import { Check } from 'lucide-svelte';
  import './lib/settings.svelte.js'; // same theme and accent as the main window

  const win = getCurrentWindow();
  let title = $state('');
  let text = $state(''); // kept between openings, so a dismissed draft isn't lost
  let saving = $state(false);
  let saved = $state(false);
  let error = $state(null);
  let box;

  const escAttr = (s) => s.replace(/&/g, '&amp;').replace(/"/g, '&quot;');

  // each line is its own paragraph; a line that is just a link becomes a link card
  const toMarkdown = (t) =>
    t.trim().split('\n').map((l) => l.trim()).filter(Boolean)
      .map((l) => (/^https?:\/\/\S+$/.test(l) ? `<div data-link-card="${escAttr(l)}">${escAttr(new URL(l).hostname.replace(/^www\./, ''))}</div>` : l))
      .join('\n\n');

  async function save() {
    if ((!text.trim() && !title.trim()) || saving) return;
    // the first line is the note's title; a bare "#" keeps it empty so the content isn't promoted to title
    const body = `# ${title.trim()}`.trim() + (text.trim() ? `

${toMarkdown(text)}` : '');
    error = null;
    try {
      await invoke('save_note', { path: null, body, created: new Date().toISOString() });
    } catch (e) {
      error = `Couldn't save: ${e}`;
      return;
    }
    await emitTo('main', 'notes-changed');
    saving = saved = true;
    setTimeout(async () => {
      await win.hide();
      text = title = '';
      saving = saved = false;
    }, 1100);
  }

  function onKey(e) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); save(); }
    else if (e.key === 'Escape') { e.preventDefault(); win.hide(); }
  }

  onMount(() => {
    const offOpen = listen('capture-open', () => { error = null; requestAnimationFrame(() => box?.focus()); });
    // clicking away puts it away, like Windows search
    const offFocus = win.onFocusChanged(({ payload: focused }) => { if (!focused && !saving) win.hide(); });
    return () => { offOpen.then((f) => f()); offFocus.then((f) => f()); };
  });
</script>

<div class="capture-stage">
  <div class="capture" class:saving role="dialog" aria-label="Quick note">
    <div class="capture-head" data-tauri-drag-region><span class="name" data-tauri-drag-region>Envelo</span><span data-tauri-drag-region>New note</span></div>
    <input class="title" bind:value={title} onkeydown={onKey} aria-label="Title" placeholder="Title (optional)">
    <!-- svelte-ignore a11y_autofocus -->
    <textarea bind:this={box} bind:value={text} onkeydown={onKey} autofocus aria-label="Quick note text" placeholder="A thought, a link, anything"></textarea>
    {#if error}<p class="capture-error" role="alert">{error}</p>{/if}
    <div class="capture-foot"><span><kbd>Enter</kbd>Save</span><span><kbd>Shift Enter</kbd>New line</span><span><kbd>Shift Tab</kbd>Title</span><span><kbd>Esc</kbd>Close</span></div>
  </div>
  <div class="saved" class:show={saved} role="status"><Check size={16} strokeWidth={1.5} />Saved to Envelo</div>
</div>
