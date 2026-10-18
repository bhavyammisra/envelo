import { mount } from 'svelte';
import '@fontsource-variable/newsreader';
import '@fontsource-variable/instrument-sans';
import './app.css';
import App from './App.svelte';

mount(App, { target: document.getElementById('app') });
