<script>
  import { onMount } from 'svelte';
  import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart';
  import { settings, set } from './settings.svelte.js';

  const inTauri = '__TAURI_INTERNALS__' in window;
  let startup = $state(false);
  let startupError = $state(null);

  onMount(async () => { if (inTauri) startup = await isEnabled().catch(() => false); });

  async function toggleStartup() {
    startupError = null;
    try {
      await (startup ? disable() : enable());
      startup = await isEnabled();
    } catch (e) {
      startupError = `Couldn't change this: ${e}`;
    }
  }

  const themes = [['system', 'System'], ['light', 'Light'], ['dark', 'Dark']];
  const styles = [['liquid', 'Liquid glass'], ['topo', 'Contour lines'], ['none', 'None']];
  const accents = [['gold', 'Gold'], ['ink', 'Ink blue'], ['terracotta', 'Terracotta'], ['moss', 'Moss']];
</script>

<div class="settings">
  <h1>Settings</h1>
  <h2>Appearance</h2>
  <div class="set-row">
    <div><div class="t">Theme</div><div class="d">System follows your Windows setting.</div></div>
    <span class="seg" role="group" aria-label="Theme">
      {#each themes as [v, label]}<button aria-pressed={settings.theme === v} onclick={() => set('theme', v)}>{label}</button>{/each}
    </span>
  </div>
  <div class="set-row">
    <div><div class="t">Selected note style</div><div class="d">How the open note stands out in the list.</div></div>
    <span class="seg" role="group" aria-label="Selected note style">
      {#each styles as [v, label]}<button aria-pressed={settings.sel === v} onclick={() => set('sel', v)}>{label}</button>{/each}
    </span>
  </div>
  <div class="set-row">
    <div><div class="t">Accent colour</div><div class="d">Used for checkboxes, buttons and highlights.</div></div>
    <div class="swatches" role="group" aria-label="Accent colour">
      {#each accents as [v, label]}<button class="swatch" aria-pressed={settings.accent === v} onclick={() => set('accent', v)}><i class="sw-{v}"></i>{label}</button>{/each}
    </div>
  </div>

  <h2>Notes</h2>
  <div class="set-row">
    <div><div class="t">Notes folder</div><div class="path">Documents\Envelo</div></div>
  </div>
  <div class="set-row">
    <div><div class="t">Quick capture shortcut</div><div class="d">Opens a small note box over any app.</div></div>
    <span class="hint"><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>Space</kbd></span>
  </div>
  <div class="set-row">
    <div>
      <div class="t">Start with Windows</div>
      <div class="d">{startupError ?? 'Waits quietly in the tray so quick capture is ready.'}</div>
    </div>
    <button class="toggle" role="switch" aria-checked={startup} aria-label="Start with Windows" disabled={!inTauri} onclick={toggleStartup}></button>
  </div>
</div>
