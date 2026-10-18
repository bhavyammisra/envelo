<script>
  import { onMount, tick } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Minus, Square, X, PanelLeft, SquarePen, Notebook, Settings as Gear, Search, Maximize2, Minimize2, Trash2 } from 'lucide-svelte';
  import Editor from './lib/Editor.svelte';
  import Settings from './lib/Settings.svelte';
  import { initMedia } from './lib/media.js';
  import { store, load, visible, current, select, create, edit, remove, flush, groupOf, timeOf, stampOf } from './lib/notes.svelte.js';

  const win = '__TAURI_INTERNALS__' in window ? getCurrentWindow() : null;
  const icon = { size: 16, strokeWidth: 1.5 };

  let view = $state('notes'); // 'notes' | 'settings'
  let focusMode = $state(false);
  let sidebar = $state(true);
  let search = $state(), listEl = $state(), lens = $state();

  const list = $derived(visible());
  const groups = $derived.by(() => {
    const out = [];
    for (const n of list) {
      const g = groupOf(n.modified);
      if (out.at(-1)?.name !== g) out.push({ name: g, notes: [] });
      out.at(-1).notes.push(n);
    }
    return out;
  });
  const note = $derived(store.notes.find((n) => n.id === store.currentId) ?? null);

  // contour patch per note: a stable offset into the map from the title
  const patch = (n) => {
    let h = 0;
    for (const ch of String(n.id) + n.created) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
    return `${-(h % 520)}px ${-((h >> 8) % 160)}px`;
  };

  // the liquid glass lens glides to the selected row, and its refraction map is sized to it
  $effect(() => {
    store.currentId; list.length; view; sidebar; focusMode;
    tick().then(() => {
      const row = listEl?.querySelector('.row[aria-current="true"]');
      if (!lens) return;
      lens.style.visibility = row ? 'visible' : 'hidden';
      if (!row) return;
      lens.style.setProperty('--sel-y', `${row.offsetTop}px`);
      lens.style.setProperty('--sel-h', `${row.offsetHeight}px`);
      const f = document.getElementById('liquid');
      [f, f.querySelector('feImage')].forEach((el) => { el.setAttribute('width', row.offsetWidth); el.setAttribute('height', row.offsetHeight); });
    });
  });

  function newNote() {
    view = 'notes';
    store.query = '';
    create();
  }

  function onKey(e) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === 'n') { e.preventDefault(); newNote(); }
    else if (mod && e.key.toLowerCase() === 'k') { e.preventDefault(); view = 'notes'; focusMode = false; tick().then(() => search?.focus()); }
    else if (mod && e.key === '.') { e.preventDefault(); focusMode = !focusMode; }
    else if (mod && e.key === '\\') { e.preventDefault(); sidebar = !sidebar; }
    else if (e.key === 'Escape' && focusMode) focusMode = false;
  }

  function listKey(e) {
    if (e.key === 'Delete' && note) { e.preventDefault(); remove(note); }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const i = list.findIndex((n) => n.id === store.currentId);
      const next = list[Math.max(0, Math.min(list.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
      if (next) select(next.id);
    }
  }

  onMount(async () => {
    try {
      await initMedia();
      await load();
    } catch (e) {
      store.error = `Couldn't open your notes folder: ${e}`;
    } finally {
      await tick();
      // shown only once the last note is in the DOM: no splash, no white flash.
      // (a hidden web view doesn't run animation frames, so this can't wait for one)
      win?.show();
    }
    addEventListener('beforeunload', () => store.notes.forEach(flush));
  });
</script>

<svelte:window onkeydown={onKey} />

<svg width="0" height="0" style="position:absolute" aria-hidden="true">
  <filter id="liquid" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse" x="0" y="0" width="300" height="64" color-interpolation-filters="sRGB">
    <feImage href="/lens-map.png" x="0" y="0" width="300" height="64" preserveAspectRatio="none" result="map" />
    <feGaussianBlur in="SourceGraphic" stdDeviation="0.35" result="soft" />
    <feDisplacementMap in="soft" in2="map" scale="14" xChannelSelector="R" yChannelSelector="G" />
  </filter>
</svg>

<div class="window">
  <div class="titlebar" data-tauri-drag-region>
    <img class="app-ico" src="/icon.svg" alt="" data-tauri-drag-region />
    <span class="name" data-tauri-drag-region>Envelo</span>
    <div class="caption">
      <button aria-label="Minimise" onclick={() => win?.minimize()}><Minus {...icon} /></button>
      <button aria-label="Maximise" onclick={() => win?.toggleMaximize()}><Square size={14} strokeWidth={1.5} /></button>
      <button class="close" aria-label="Close" onclick={() => win?.close()}><X {...icon} /></button>
    </div>
  </div>

  <div class="app" class:focus={focusMode && view === 'notes'} style:grid-template-columns={view === 'settings' ? '216px minmax(0, 1fr)' : sidebar ? null : '0 304px minmax(0, 1fr)'}>
    <nav class="sidebar" aria-label="Library" style:visibility={sidebar || view === 'settings' ? null : 'hidden'}>
      <div class="side-top">
        <button class="icon-btn" aria-label="Hide sidebar (Ctrl+\)" title="Hide sidebar  Ctrl+\" onclick={() => (sidebar = false)}><PanelLeft {...icon} /></button>
        <button class="icon-btn" aria-label="New note (Ctrl+N)" title="New note  Ctrl+N" onclick={newNote}><SquarePen {...icon} /></button>
      </div>
      <button class="nav" aria-current={view === 'notes'} onclick={() => (view = 'notes')}><Notebook {...icon} />All notes<span class="count">{store.notes.filter((n) => n.path || n.body).length}</span></button>
      <div class="side-foot"><button class="nav" aria-current={view === 'settings'} onclick={() => (view = 'settings')}><Gear {...icon} />Settings</button></div>
    </nav>

    {#if view === 'notes'}
      <section class="list" aria-label="Notes">
        <div class="list-top">
          {#if !sidebar}<button class="icon-btn" aria-label="Show sidebar" onclick={() => (sidebar = true)}><PanelLeft {...icon} /></button>{/if}
          <label class="search"><Search {...icon} /><input bind:this={search} bind:value={store.query} placeholder="Search notes" aria-label="Search notes" /><kbd>Ctrl K</kbd></label>
        </div>
        <div class="list-scroll" bind:this={listEl} role="listbox" tabindex="0" aria-label="Notes" onkeydown={listKey}>
          <div class="lens" aria-hidden="true" bind:this={lens}></div>
          {#each groups as g (g.name)}
            <div class="group">{g.name}</div>
            {#each g.notes as n (n.id)}
              <button class="row" role="option" aria-selected={n.id === store.currentId} aria-current={n.id === store.currentId} onclick={() => select(n.id)}>
                {#if n.id === store.currentId}{#key n.id}<div class="topo" aria-hidden="true" style:--pos={patch(n)}></div>{/key}{/if}
                <div class="row-title">{n.title || 'New note'}</div>
                <div class="row-meta"><time>{timeOf(n.modified)}</time><span>{n.preview}</span></div>
              </button>
            {/each}
          {:else}
            <p class="empty">{store.query ? `No notes mention "${store.query}".` : 'Press Ctrl+N to start a note.'}</p>
          {/each}
        </div>
      </section>

      <section class="editor" aria-label="Note">
        {#if note}
          <div class="editor-bar">
            <button class="icon-btn" aria-label="Focus mode (Ctrl+.)" title="Focus mode  Ctrl+." onclick={() => (focusMode = !focusMode)}>
              {#if focusMode}<Minimize2 {...icon} />{:else}<Maximize2 {...icon} />{/if}
            </button>
            <button class="icon-btn" aria-label="Move to Recently deleted" title="Delete note" onclick={() => remove(note)}><Trash2 {...icon} /></button>
          </div>
          <article class="doc">
            <div class="stamp">{stampOf(note)}</div>
            <Editor {note} onchange={(md) => edit(note, md)} />
          </article>
        {:else if store.loaded}
          <div class="doc-empty">Press Ctrl+N to start a note.</div>
        {/if}
      </section>
    {:else}
      <section class="editor" aria-label="Settings"><Settings /></section>
    {/if}
  </div>
</div>

{#if store.error}
  <div class="toast" role="alert">{store.error}<button class="ghost-btn" onclick={() => (store.error = null)}>Dismiss</button></div>
{/if}
