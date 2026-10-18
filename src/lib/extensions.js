import { Node, Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import Image from '@tiptap/extension-image';
import Suggestion from '@tiptap/suggestion';
import { resolveSrc, hostOf, openLink } from './media.js';

const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** Pictures keep their relative path (`images/x.png`) in the note and are shown through the asset protocol. */
export const NoteImage = Image.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      src: {
        default: null,
        parseHTML: (el) => el.getAttribute('data-src') ?? el.getAttribute('src'),
        renderHTML: (a) => ({ src: resolveSrc(a.src), 'data-src': a.src }),
      },
    };
  },
  // pictures are blocks: each gets its own line in the .md file
  addStorage: () => ({
    markdown: {
      serialize(state, node) {
        state.write(`![${state.esc(node.attrs.alt ?? '')}](${node.attrs.src})`);
        state.closeBlock(node);
      },
      parse: {},
    },
  }),
}).configure({ inline: false, HTMLAttributes: { loading: 'lazy', class: 'note-img' } });

/**
 * A pasted link on its own line becomes a card with the site name and, once fetched, the page title.
 * Saved in the .md file as `<div data-link-card="url">Title</div>` so it reads back as a card.
 */
export const LinkCard = Node.create({
  name: 'linkCard',
  group: 'block',
  atom: true,
  draggable: true,
  addAttributes: () => ({ href: { default: null }, title: { default: null } }),
  parseHTML: () => [{ tag: 'div[data-link-card]', getAttrs: (el) => ({ href: el.getAttribute('data-link-card'), title: el.textContent.trim() || null }) }],
  renderHTML: ({ node }) => ['div', { 'data-link-card': node.attrs.href }, node.attrs.title ?? hostOf(node.attrs.href)],
  addNodeView() {
    return ({ node }) => {
      const dom = document.createElement('a');
      dom.className = 'linkline';
      dom.href = node.attrs.href;
      dom.title = `${node.attrs.href}\nClick to open`;
      const host = hostOf(node.attrs.href);
      dom.innerHTML = `<span class="fav">${esc(host[0]?.toUpperCase() ?? '?')}</span><span><b>${esc(node.attrs.title ?? host)}</b><small>${esc(host)}</small></span>`;
      dom.addEventListener('click', (e) => { e.preventDefault(); openLink(node.attrs.href); });
      return { dom };
    };
  },
  addStorage: () => ({
    markdown: {
      serialize(state, node) {
        state.write(`<div data-link-card="${esc(node.attrs.href)}">${esc(node.attrs.title ?? hostOf(node.attrs.href))}</div>`);
        state.closeBlock(node);
      },
      parse: {},
    },
  }),
});

/** The `/` menu. `onState` receives { items, index, rect, pick } while open and null when closed. */
export const SlashMenu = Extension.create({
  name: 'slashMenu',
  addOptions: () => ({ items: [], onState: () => {} }),
  addProseMirrorPlugins() {
    const { items, onState } = this.options;
    let state = null;
    return [
      Suggestion({
        editor: this.editor,
        char: '/',
        items: ({ query }) => items.filter((i) => (i.label + ' ' + (i.keys ?? '')).toLowerCase().includes(query.toLowerCase())),
        command: ({ editor, range, props }) => props.run(editor.chain().focus().deleteRange(range), editor),
        render: () => ({
          onStart: (p) => { state = { items: p.items, index: 0, rect: p.clientRect?.(), pick: (i) => p.command(p.items[i]) }; onState(state); },
          onUpdate: (p) => { state = { items: p.items, index: 0, rect: p.clientRect?.(), pick: (i) => p.command(p.items[i]) }; onState(state); },
          onKeyDown: ({ event }) => {
            if (!state?.items.length) return false;
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              const n = state.items.length;
              state = { ...state, index: (state.index + (event.key === 'ArrowDown' ? 1 : n - 1)) % n };
              onState(state);
              return true;
            }
            if (event.key === 'Enter' || event.key === 'Tab') { state.pick(state.index); return true; }
            if (event.key === 'Escape') { onState(null); return true; }
            return false;
          },
          onExit: () => { state = null; onState(null); },
        }),
      }),
    ];
  },
});

/** Keeps an empty paragraph after a picture, table or card at the end, so there's always somewhere to type. */
export const TrailingParagraph = Extension.create({
  name: 'trailingParagraph',
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey('trailingParagraph'),
        appendTransaction: (trs, __, state) => {
          const last = state.doc.lastChild;
          if (!last || last.type.name === 'paragraph') return null;
          const tr = state.tr.insert(state.doc.content.size, state.schema.nodes.paragraph.create());
          // opening a note must not count as an edit
          if (trs.some((t) => t.getMeta('preventUpdate'))) tr.setMeta('preventUpdate', true);
          return tr;
        },
      }),
    ];
  },
});
