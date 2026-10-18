import { mount } from 'svelte';
import '@fontsource-variable/newsreader';
import '@fontsource-variable/instrument-sans';
import './app.css';

// the same page serves the main window and the small quick-capture window
const capture = new URLSearchParams(location.search).get('view') === 'capture';
document.documentElement.classList.toggle('capture-window', capture);

const { default: Root } = capture ? await import('./Capture.svelte') : await import('./App.svelte');
mount(Root, { target: document.getElementById('app') });
