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
  import { Bold, Italic, Underline as U, Strikethrough, Link as LinkIcon } from 'lucide-svelte';

  let { note, onchange } = $props();

  let host, bar;
  let editor = $state(null);
  let tick = $state(0); // bumps on selection change so the format bar shows active states
  let shownId = null;

  // Ctrl+1/2/3 for headings, Ctrl+0 back to body text
  const HeadingKeys = Extension.create({
    name: 'headingKeys',
    addKeyboardShortcuts() {
      const h = (level) => () => this.editor.chain().focus().toggleHeading({ level }).run();
      return { 'Mod-1': h(1), 'Mod-2': h(2), 'Mod-3': h(3), 'Mod-0': () => this.editor.chain().focus().setParagraph().run() };
    },
  });

  onMount(() => {
    editor = new Editor({
      element: host,
      extensions: [
        StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
        Placeholder.configure({ placeholder: ({ pos }) => (pos === 0 ? 'Title' : 'Start writing') }),
        Underline, Highlight, Typography, HeadingKeys,
        TaskList, TaskItem.configure({ nested: true }),
        Link.configure({ openOnClick: false, autolink: true, HTMLAttributes: { rel: 'noopener noreferrer' } }),
        Table.configure({ resizable: false }), TableRow, TableHeader, TableCell,
        BubbleMenu.configure({ element: bar, tippyOptions: { duration: 120, placement: 'top' } }),
        Markdown.configure({ html: true, tightLists: true, linkify: true, transformPastedText: true }),
      ],
      content: note?.body ?? '',
      onUpdate: ({ editor }) => onchange(editor.storage.markdown.getMarkdown()),
      onSelectionUpdate: () => tick++,
      onTransaction: () => tick++,
    });
    shownId = note?.id;
    if (!note?.body) editor.commands.focus();
  });

  onDestroy(() => editor?.destroy());

  // switching notes swaps the document without firing an edit
  $effect(() => {
    const n = note;
    if (!editor || !n || n.id === shownId) return;
    shownId = n.id;
    editor.commands.setContent(n.body, false);
    editor.commands.focus(n.body ? 'start' : null);
  });

  const is = (name, attrs) => (tick, editor?.isActive(name, attrs));
  const run = (fn) => fn(editor.chain().focus()).run();

  function link() {
    if (editor.isActive('link')) return run((c) => c.unsetLink());
    const url = prompt('Link address');
    if (url) run((c) => c.extendMarkRange('link').setLink({ href: /^[a-z]+:/i.test(url) ? url : `https://${url}` }));
  }

  export function focus() { editor?.commands.focus(); }
</script>

<div class="fmt" role="toolbar" aria-label="Format" bind:this={bar}>
  {#if editor}
    <button aria-pressed={is('heading', { level: 1 })} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleHeading({ level: 1 }))}>H1</button>
    <button aria-pressed={is('heading', { level: 2 })} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleHeading({ level: 2 }))}>H2</button>
    <button aria-pressed={is('heading', { level: 3 })} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleHeading({ level: 3 }))}>H3</button>
    <span class="sep"></span>
    <button aria-label="Bold" aria-pressed={is('bold')} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleBold())}><Bold size={16} strokeWidth={1.5} /></button>
    <button aria-label="Italic" aria-pressed={is('italic')} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleItalic())}><Italic size={16} strokeWidth={1.5} /></button>
    <button aria-label="Underline" aria-pressed={is('underline')} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleUnderline())}><U size={16} strokeWidth={1.5} /></button>
    <button aria-label="Strikethrough" aria-pressed={is('strike')} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleStrike())}><Strikethrough size={16} strokeWidth={1.5} /></button>
    <span class="sep"></span>
    <button aria-label="Highlight" aria-pressed={is('highlight')} onmousedown={(e) => e.preventDefault()} onclick={() => run((c) => c.toggleHighlight())}><span class="dot"></span></button>
    <button aria-label="Link" aria-pressed={is('link')} onmousedown={(e) => e.preventDefault()} onclick={link}><LinkIcon size={16} strokeWidth={1.5} /></button>
  {/if}
</div>
<div class="prose" bind:this={host}></div>

<style>
  /* the format bar is positioned by the bubble menu, not by the mockup's absolute rule */
  .fmt { position: static; transform: none; bottom: auto; left: auto; }
</style>
