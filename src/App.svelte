<script>
  import { onMount, tick } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    Minus, Square, X, PanelLeft, SquarePen, Notebook, Settings as Gear, Search, Maximize2, Minimize2, Trash2,
    Pin, PinOff, Folder, FolderInput, Plus, RotateCcw, Check,
  } from 'lucide-svelte';
  import Editor from './lib/Editor.svelte';
  import Settings from './lib/Settings.svelte';
  import { initMedia } from './lib/media.js';
  import {
    store, load, refresh, flushAll, visible, select, create, edit, remove, flush, groupOf, timeOf, stampOf,
    togglePin, moveTo, newFolder, renameFolder, deleteFolder, loadTrash, restore, deleteForever, daysLeft,
  } from './lib/notes.svelte.js';

  const win = '__TAURI_INTERNALS__' in window ? getCurrentWindow() : null;
  const icon = { size: 16, strokeWidth: 1.5 };

  let page = $state('notes'); // 'notes' | 'settings'
  let focusMode = $state(false);
  let sidebar = $state(true);
  let search = $state(), listEl = $state(), lens = $state();
  let naming = $state(null); // { from: null | folder, value } while creating or renaming a folder
  let confirmFolder = $state(null); // folder waiting for a second click to remove
  let confirmForever = $state(false);
  let moveMenu = $state(false);

  const inTrash = $derived(store.view === 'trash');
  const matches = (n, words) => words.every((w) => (n.title + '\n' + n.body).toLowerCase().includes(w));
  const items = $derived.by(() => {
    if (!inTrash) return visible();
    const words = store.query.toLowerCase().split(/\s+/).filter(Boolean);
    return store.trash.filter((t) => matches(t, words));
  });
  const currentKey = $derived(inTrash ? store.trashId : store.currentId);
  const groups = $derived.by(() => {
    const out = [];
    const add = (name, n) => { if (out.at(-1)?.name !== name) out.push({ name, notes: [] }); out.at(-1).notes.push(n); };
    const pinnedFirst = store.view === 'all' && !store.query;
    if (pinnedFirst) items.filter((n) => n.pinned).forEach((n) => add('Pinned', n));
    for (const n of items) if (!(pinnedFirst && n.pinned)) add(groupOf(n.modified), n);
    return out;
  });
  const note = $derived(store.notes.find((n) => n.id === store.currentId) ?? null);
  const trashed = $derived(store.trash.find((t) => t.id === store.trashId) ?? null);
  const count = (folder) => store.notes.filter((n) => n.folder === folder).length;
  const saved = (n) => n.path || n.body.trim();

  // contour patch per note: a stable offset into the map
  const patch = (n) => {
    let h = 0;
    for (const ch of String(n.id) + n.created) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
    return `${-(h % 520)}px ${-((h >> 8) % 160)}px`;
  };

  // the liquid glass lens glides to the selected row, and its refraction map is sized to it
  $effect(() => {
    currentKey; items.length; page; sidebar; focusMode; store.view;
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

  async function show(view) {
    page = 'notes';
    moveMenu = false;
    confirmForever = false;
    store.view = view;
    if (view === 'trash') await loadTrash();
    else if (!items.some((n) => n.id === store.currentId)) store.currentId = items[0]?.id ?? null;
  }

  function newNote() {
    if (store.view === 'trash' || store.view === 'pinned') store.view = 'all';
    page = 'notes';
    store.query = '';
    create();
  }

  function pick(n) {
    if (inTrash) { store.trashId = n.id; confirmForever = false; }
    else select(n.id);
  }

  async function finishNaming() {
    const n = naming;
    naming = null;
    const value = n?.value.trim();
    if (!value || value === n.from) return;
    if (n.from) await renameFolder(n.from, value);
    else { const made = await newFolder(value); if (made) show(`folder:${made}`); }
  }

  function onKey(e) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === 'n') { e.preventDefault(); newNote(); }
    else if (mod && e.key.toLowerCase() === 'k') { e.preventDefault(); page = 'notes'; focusMode = false; tick().then(() => search?.focus()); }
    else if (mod && e.key === '.') { e.preventDefault(); focusMode = !focusMode; }
    else if (mod && e.key === '\\') { e.preventDefault(); sidebar = !sidebar; }
    else if (e.key === 'Escape') { if (moveMenu) moveMenu = false; else if (focusMode) focusMode = false; }
  }

  function listKey(e) {
    if (e.key === 'Delete' && note && !inTrash) { e.preventDefault(); remove(note); }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const flat = groups.flatMap((g) => g.notes);
      const i = flat.findIndex((n) => n.id === currentKey);
      const next = flat[Math.max(0, Math.min(flat.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
      if (next) pick(next);
    }
  }

  const focusInput = (el) => { el.focus(); el.select(); };

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
      if (win && !(await invoke('started_hidden'))) win.show();
    }
    if (win) {
      listen('notes-changed', () => refresh());
      // Quit from the tray: save anything still pending, then exit
      listen('quit-requested', async () => { await flushAll(); invoke('quit'); });
    }
    addEventListener('beforeunload', () => store.notes.forEach(flush));
  });
</script>

<svelte:window onkeydown={onKey} onclick={(e) => { if (moveMenu && !e.target.closest?.('.move')) moveMenu = false; }} />

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
      <button class="close" aria-label="Close to tray" title="Close (Envelo keeps running in the tray)" onclick={() => flushAll().then(() => win?.close())}><X {...icon} /></button>
    </div>
  </div>

  <div class="app" class:focus={focusMode && page === 'notes'} style:grid-template-columns={page === 'settings' ? '216px minmax(0, 1fr)' : sidebar ? null : '0 304px minmax(0, 1fr)'}>
    <nav class="sidebar" aria-label="Library" style:visibility={sidebar || page === 'settings' ? null : 'hidden'}>
      <div class="side-top">
        <button class="icon-btn" aria-label="Hide sidebar (Ctrl+\)" title="Hide sidebar  Ctrl+\" onclick={() => (sidebar = false)}><PanelLeft {...icon} /></button>
        <button class="icon-btn" aria-label="New note (Ctrl+N)" title="New note  Ctrl+N" onclick={newNote}><SquarePen {...icon} /></button>
      </div>
      <button class="nav" aria-current={page === 'notes' && store.view === 'all'} onclick={() => show('all')}><Notebook {...icon} />All notes<span class="count">{store.notes.filter(saved).length}</span></button>
      <button class="nav" aria-current={page === 'notes' && store.view === 'pinned'} onclick={() => show('pinned')}><Pin {...icon} />Pinned<span class="count">{store.notes.filter((n) => n.pinned).length}</span></button>

      <div class="side-head">Folders<button onclick={() => (naming = { from: null, value: '' })}>New</button></div>
      {#each store.folders as f (f)}
        {#if naming?.from === f}
          <input class="nav-input" use:focusInput bind:value={naming.value} aria-label="Folder name"
            onkeydown={(e) => { if (e.key === 'Enter') finishNaming(); if (e.key === 'Escape') naming = null; }} onblur={finishNaming} />
        {:else}
          <div class="folder-row">
            <button class="nav" aria-current={page === 'notes' && store.view === `folder:${f}`} onclick={() => show(`folder:${f}`)}
              ondblclick={() => (naming = { from: f, value: f })} title="Double-click to rename">
              <Folder {...icon} /><span class="label">{f}</span><span class="count">{count(f)}</span>
            </button>
            {#if confirmFolder === f}
              <button class="folder-x confirm" onclick={() => { confirmFolder = null; deleteFolder(f); }} onblur={() => (confirmFolder = null)} title="Its notes move to All notes">Remove</button>
            {:else}
              <button class="folder-x" aria-label="Remove folder {f}" title="Remove folder" onclick={() => (confirmFolder = f)}><X size={14} strokeWidth={1.5} /></button>
            {/if}
          </div>
        {/if}
      {/each}
      {#if naming && !naming.from}
        <input class="nav-input" use:focusInput bind:value={naming.value} placeholder="Folder name" aria-label="New folder name"
          onkeydown={(e) => { if (e.key === 'Enter') finishNaming(); if (e.key === 'Escape') naming = null; }} onblur={finishNaming} />
      {/if}

      <div class="side-head">Private</div>
      <button class="nav" aria-current={page === 'notes' && inTrash} onclick={() => show('trash')}><Trash2 {...icon} />Recently deleted</button>
      <div class="side-foot"><button class="nav" aria-current={page === 'settings'} onclick={() => (page = 'settings')}><Gear {...icon} />Settings</button></div>
    </nav>

    {#if page === 'notes'}
      <section class="list" aria-label="Notes">
        <div class="list-top">
          {#if !sidebar}<button class="icon-btn" aria-label="Show sidebar" onclick={() => (sidebar = true)}><PanelLeft {...icon} /></button>{/if}
          <label class="search"><Search {...icon} /><input bind:this={search} bind:value={store.query} placeholder={inTrash ? 'Search deleted notes' : 'Search notes'} aria-label="Search notes" /><kbd>Ctrl K</kbd></label>
        </div>
        <div class="list-scroll" bind:this={listEl} role="listbox" tabindex="0" aria-label="Notes" onkeydown={listKey}>
          <div class="lens" aria-hidden="true" bind:this={lens}></div>
          {#each groups as g (g.name)}
            <div class="group">{g.name}</div>
            {#each g.notes as n (n.id)}
              <button class="row" role="option" aria-selected={n.id === currentKey} aria-current={n.id === currentKey} onclick={() => pick(n)}>
                {#if n.id === currentKey}{#key n.id}<div class="topo" aria-hidden="true" style:--pos={patch(n)}></div>{/key}{/if}
                <div class="row-title">{#if n.pinned}<Pin size={13} strokeWidth={1.5} />{/if}{n.title || 'New note'}</div>
                <div class="row-meta"><time>{timeOf(n.modified)}</time><span>{n.preview}</span></div>
              </button>
            {/each}
          {:else}
            <p class="empty">
              {#if store.query}No notes mention "{store.query}".
              {:else if inTrash}Deleted notes wait here for 30 days.
              {:else if store.view === 'pinned'}Pin a note to keep it at the top.
              {:else if store.view.startsWith('folder:')}Press Ctrl+N to start a note in this folder.
              {:else}Press Ctrl+N to start a note.{/if}
            </p>
          {/each}
        </div>
      </section>

      <section class="editor" aria-label="Note">
        {#if inTrash}
          {#if trashed}
            <div class="trash-bar">
              <span>Deleted {stampOf({ created: trashed.deleted, modified: trashed.deleted })}. Removed for good in {daysLeft(trashed)} {daysLeft(trashed) === 1 ? 'day' : 'days'}.</span>
              <button class="ghost-btn" onclick={() => restore(trashed)}><RotateCcw size={14} strokeWidth={1.5} />Restore</button>
              {#if confirmForever}
                <button class="ghost-btn danger" onclick={() => { confirmForever = false; deleteForever(trashed); }}>Delete forever</button>
              {:else}
                <button class="ghost-btn" onclick={() => (confirmForever = true)}>Delete now</button>
              {/if}
            </div>
            <article class="doc"><Editor note={trashed} readonly /></article>
          {/if}
        {:else if note}
          <div class="editor-bar">
            <button class="icon-btn" aria-label={note.pinned ? 'Unpin note' : 'Pin note'} aria-pressed={note.pinned} title={note.pinned ? 'Unpin' : 'Pin to the top'} onclick={() => togglePin(note)}>
              {#if note.pinned}<PinOff {...icon} />{:else}<Pin {...icon} />{/if}
            </button>
            <div class="move">
              <button class="icon-btn" aria-label="Move to folder" aria-expanded={moveMenu} title="Move to folder" onclick={() => (moveMenu = !moveMenu)}><FolderInput {...icon} /></button>
              {#if moveMenu}
                <div class="menu" role="menu">
                  <button role="menuitem" onclick={() => { moveMenu = false; moveTo(note, null); }}>{#if !note.folder}<Check size={14} strokeWidth={1.5} />{:else}<span class="ck"></span>{/if}No folder</button>
                  {#each store.folders as f (f)}
                    <button role="menuitem" onclick={() => { moveMenu = false; moveTo(note, f); }}>{#if note.folder === f}<Check size={14} strokeWidth={1.5} />{:else}<span class="ck"></span>{/if}{f}</button>
                  {/each}
                  <button role="menuitem" class="muted" onclick={() => { moveMenu = false; sidebar = true; naming = { from: null, value: '' }; }}><Plus size={14} strokeWidth={1.5} />New folder</button>
                </div>
              {/if}
            </div>
            <button class="icon-btn" aria-label="Focus mode (Ctrl+.)" title="Focus mode  Ctrl+." onclick={() => (focusMode = !focusMode)}>
              {#if focusMode}<Minimize2 {...icon} />{:else}<Maximize2 {...icon} />{/if}
            </button>
            <button class="icon-btn" aria-label="Move to Recently deleted" title="Delete note" onclick={() => remove(note)}><Trash2 {...icon} /></button>
          </div>
          <article class="doc">
            <div class="stamp">{stampOf(note)}{#if note.folder} · {note.folder}{/if}</div>
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
