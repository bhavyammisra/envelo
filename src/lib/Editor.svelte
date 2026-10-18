<script>
  import { onMount, onDestroy } from 'svelte';
  import { Editor, Extension } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Placeholder from '@tiptap/extension-placeholder';
  import Underline from '@tiptap/extension-underline';
  import Highlight from '@tiptap/extension-highlight';
  import TaskList from '@tiptap/extension-task-list';
  import TaskItem from '@tiptap/extension-task-item';
  import Typography from '@tiptap/extension-typography';
  import Link from '@tiptap/extension-link';
  import Table from '@tiptap/extension-table';
  import TableRow from '@tiptap/extension-table-row';
  import TableCell from '@tiptap/extension-table-cell';
  import TableHeader from '@tiptap/extension-table-header';
  import BubbleMenu from '@tiptap/extension-bubble-menu';
  import { Markdown } from 'tiptap-markdown';
  import {
    Bold, Italic, Underline as U, Strikethrough, Link as LinkIcon,
    Heading1, Heading2, Heading3, Pilcrow, List, ListOrdered, ListChecks, Quote, Minus, Table as TableIcon, Image as ImageIcon,
  } from 'lucide-svelte';
  import { NoteImage, LinkCard, SlashMenu, TrailingParagraph } from './extensions.js';
  import { savePicture, isPicture, fetchTitle, openLink, hostOf } from './media.js';

  let { note, onchange = () => {}, readonly = false, pictures = true, onnotice = () => {} } = $props();

  let host, bar, tableBar, picker;
  let editor = $state(null);
  let tick = $state(0); // bumps on selection change so the bars show active states
  let slash = $state(null); // the open / menu
  let viewer = $state(null); // picture shown larger
  let shownId = null;
  const icon = { size: 16, strokeWidth: 1.5 };

  // Ctrl+1/2/3 for headings, Ctrl+0 back to body text
  const HeadingKeys = Extension.create({
    name: 'headingKeys',
    addKeyboardShortcuts() {
      const h = (level) => () => this.editor.chain().focus().toggleHeading({ level }).run();
      return { 'Mod-1': h(1), 'Mod-2': h(2), 'Mod-3': h(3), 'Mod-0': () => this.editor.chain().focus().setParagraph().run() };
    },
  });

  const slashItems = [
    { label: 'Heading 1', keys: 'h1 title', icon: Heading1, run: (c) => c.setHeading({ level: 1 }).run() },
    { label: 'Heading 2', keys: 'h2', icon: Heading2, run: (c) => c.setHeading({ level: 2 }).run() },
    { label: 'Heading 3', keys: 'h3', icon: Heading3, run: (c) => c.setHeading({ level: 3 }).run() },
    { label: 'Text', keys: 'body paragraph', icon: Pilcrow, run: (c) => c.setParagraph().run() },
    { label: 'Bullets', keys: 'list ul', icon: List, run: (c) => c.toggleBulletList().run() },
    { label: 'Numbers', keys: 'list ol numbered', icon: ListOrdered, run: (c) => c.toggleOrderedList().run() },
    { label: 'Checklist', keys: 'todo task checkbox', icon: ListChecks, run: (c) => c.toggleTaskList().run() },
    { label: 'Quote', keys: 'blockquote', icon: Quote, run: (c) => c.toggleBlockquote().run() },
    { label: 'Divider', keys: 'hr line rule', icon: Minus, run: (c) => c.setHorizontalRule().run() },
    { label: 'Table', keys: 'grid', icon: TableIcon, run: (c) => c.insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run() },
    { label: 'Picture', keys: 'image photo screenshot', icon: ImageIcon, run: (c) => { c.run(); picker.click(); } },
  ].filter((i) => pictures || i.label !== 'Picture');

  async function addPictures(files, pos) {
    // pictures are saved as ordinary files, so they'd leak out of an encrypted note
    if (!pictures) return onnotice("Pictures aren't encrypted, so they can't go in locked notes.");
    for (const file of files) {
      try {
        const src = await savePicture(file);
        const content = { type: 'image', attrs: { src } };
        if (pos == null) editor.chain().focus().insertContent(content).run();
        else editor.chain().focus().insertContentAt(pos, content).run();
      } catch (e) {
        console.error('Could not add picture', e);
      }
    }
  }

  // cards saved offline only know the site name; fill in real titles when we can
  function refreshCards() {
    const missing = new Set();
    editor.state.doc.descendants((n) => {
      if (n.type.name === 'linkCard' && (!n.attrs.title || n.attrs.title === hostOf(n.attrs.href))) missing.add(n.attrs.href);
    });
    for (const href of missing) {
      fetchTitle(href).then((title) => {
        if (!title || !editor) return;
        const tr = editor.state.tr;
        editor.state.doc.descendants((n, pos) => {
          if (n.type.name === 'linkCard' && n.attrs.href === href && n.attrs.title !== title) tr.setNodeMarkup(pos, undefined, { ...n.attrs, title });
        });
        if (tr.docChanged) editor.view.dispatch(tr);
      });
    }
  }

  onMount(() => {
    editor = new Editor({
      element: host,
      extensions: [
        StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
        Placeholder.configure({ placeholder: ({ pos }) => (pos === 0 ? 'Title' : 'Start writing, or type / for blocks') }),
        Underline, Highlight, Typography, HeadingKeys,
        TaskList, TaskItem.configure({ nested: true }),
        Link.configure({ openOnClick: false, autolink: true, HTMLAttributes: { rel: 'noopener noreferrer' } }),
        Table.configure({ resizable: false }), TableRow, TableHeader, TableCell,
        NoteImage, LinkCard, TrailingParagraph,
        SlashMenu.configure({ items: slashItems, onState: (s) => (slash = s) }),
        BubbleMenu.configure({
          pluginKey: 'formatBar', element: bar, tippyOptions: { duration: 120, placement: 'top', maxWidth: 'none' },
          shouldShow: ({ state }) => !state.selection.empty && !state.selection.node,
        }),
        BubbleMenu.configure({
          pluginKey: 'tableBar', element: tableBar,
          // sits just above the whole table instead of over the text cursor
          tippyOptions: { duration: 120, placement: 'top-start', maxWidth: 'none', offset: [0, 8], getReferenceClientRect: tableRect },
          shouldShow: ({ editor, state }) => state.selection.empty && editor.isActive('table'),
        }),
        Markdown.configure({ html: true, tightLists: true, linkify: true, transformPastedText: true }),
      ],
      editorProps: {
        handlePaste(view, event) {
          const files = [...(event.clipboardData?.files ?? [])].filter(isPicture);
          if (files.length) { addPictures(files); return true; }
          const text = event.clipboardData?.getData('text/plain')?.trim() ?? '';
          const sel = view.state.selection, block = sel.$from.parent;
          if (/^https?:\/\/\S+$/.test(text) && sel.empty && block.type.name === 'paragraph' && !block.textContent) {
            editor.chain().focus().insertContent({ type: 'linkCard', attrs: { href: text, title: null } }).run();
            refreshCards();
            return true;
          }
          return false;
        },
        handleDrop(view, event) {
          const files = [...(event.dataTransfer?.files ?? [])].filter(isPicture);
          if (!files.length) return false;
          event.preventDefault();
          addPictures(files, view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos);
          return true;
        },
        handleClick(view, pos, event) {
          const a = event.target.closest?.('a[href]');
          if (a && (event.ctrlKey || event.metaKey) && !a.classList.contains('linkline')) { openLink(a.href); return true; }
          return false;
        },
      },
      editable: !readonly,
      content: note?.body ?? '',
      onUpdate: ({ editor }) => {
        const md = editor.storage.markdown.getMarkdown();
        if (md !== note?.body) onchange(md);
      },
      onSelectionUpdate: () => tick++,
      onTransaction: () => tick++,
    });
    shownId = note?.id;
    refreshCards();
    if (!note?.body && !readonly) editor.commands.focus();
  });

  onDestroy(() => editor?.destroy());

  // switching notes swaps the document without firing an edit
  $effect(() => {
    const n = note;
    if (!editor || !n || n.id === shownId) return;
    shownId = n.id;
    slash = null;
    editor.commands.setContent(n.body, false);
    if (!readonly) editor.commands.focus(n.body ? 'start' : null);
    refreshCards();
  });

  function tableRect() {
    const dom = editor?.view.domAtPos(editor.state.selection.from).node;
    const el = (dom?.nodeType === 1 ? dom : dom?.parentElement)?.closest('table');
    return el?.getBoundingClientRect() ?? new DOMRect();
  }

  const is = (name, attrs) => (tick, editor?.isActive(name, attrs));
  const run = (fn) => fn(editor.chain().focus()).run();
  const keep = (e) => e.preventDefault(); // buttons must not steal the editor's selection

  function link() {
    if (editor.isActive('link')) return run((c) => c.unsetLink());
    const url = prompt('Link address');
    if (url) run((c) => c.extendMarkRange('link').setLink({ href: /^[a-z]+:/i.test(url) ? url : `https://${url}` }));
  }

  function openPicture(e) {
    const img = e.target.closest?.('img.note-img');
    if (img) viewer = img.currentSrc || img.src;
  }
</script>

<div class="fmt" role="toolbar" aria-label="Format" bind:this={bar}>
  {#if editor}
    <button aria-pressed={is('heading', { level: 1 })} onmousedown={keep} onclick={() => run((c) => c.toggleHeading({ level: 1 }))}>H1</button>
    <button aria-pressed={is('heading', { level: 2 })} onmousedown={keep} onclick={() => run((c) => c.toggleHeading({ level: 2 }))}>H2</button>
    <button aria-pressed={is('heading', { level: 3 })} onmousedown={keep} onclick={() => run((c) => c.toggleHeading({ level: 3 }))}>H3</button>
    <span class="sep"></span>
    <button aria-label="Bold" aria-pressed={is('bold')} onmousedown={keep} onclick={() => run((c) => c.toggleBold())}><Bold {...icon} /></button>
    <button aria-label="Italic" aria-pressed={is('italic')} onmousedown={keep} onclick={() => run((c) => c.toggleItalic())}><Italic {...icon} /></button>
    <button aria-label="Underline" aria-pressed={is('underline')} onmousedown={keep} onclick={() => run((c) => c.toggleUnderline())}><U {...icon} /></button>
    <button aria-label="Strikethrough" aria-pressed={is('strike')} onmousedown={keep} onclick={() => run((c) => c.toggleStrike())}><Strikethrough {...icon} /></button>
    <span class="sep"></span>
    <button aria-label="Highlight" aria-pressed={is('highlight')} onmousedown={keep} onclick={() => run((c) => c.toggleHighlight())}><span class="dot"></span></button>
    <button aria-label="Link" aria-pressed={is('link')} onmousedown={keep} onclick={link}><LinkIcon {...icon} /></button>
  {/if}
</div>

<div class="fmt table-bar" role="toolbar" aria-label="Table" bind:this={tableBar}>
  {#if editor}
    <button onmousedown={keep} onclick={() => run((c) => c.addRowAfter())}>Add row</button>
    <button onmousedown={keep} onclick={() => run((c) => c.deleteRow())}>Remove row</button>
    <span class="sep"></span>
    <button onmousedown={keep} onclick={() => run((c) => c.addColumnAfter())}>Add column</button>
    <button onmousedown={keep} onclick={() => run((c) => c.deleteColumn())}>Remove column</button>
    <span class="sep"></span>
    <button onmousedown={keep} onclick={() => run((c) => c.deleteTable())}>Delete table</button>
  {/if}
</div>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="prose" bind:this={host} onclick={openPicture}></div>

<input bind:this={picker} type="file" accept="image/png,image/jpeg,image/gif,image/webp" multiple hidden
  onchange={(e) => { addPictures([...e.currentTarget.files]); e.currentTarget.value = ''; }} />

{#if slash?.items.length}
  <div class="slash" role="listbox" aria-label="Insert block" style:left="{slash.rect?.left ?? 0}px" style:top="{(slash.rect?.bottom ?? 0) + 6}px">
    {#each slash.items as item, i (item.label)}
      <button role="option" aria-selected={i === slash.index} onmousedown={keep} onclick={() => slash.pick(i)}>
        <item.icon {...icon} />{item.label}
      </button>
    {/each}
  </div>
{/if}

{#if viewer}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="viewer" onclick={() => (viewer = null)}>
    <img src={viewer} alt="" />
  </div>
{/if}
<svelte:window onkeydown={(e) => e.key === 'Escape' && viewer && (viewer = null)} />

<style>
  /* bars are positioned by the bubble menu, not by the mockup's absolute rule */
  .fmt { position: static; transform: none; bottom: auto; left: auto; }
</style>
