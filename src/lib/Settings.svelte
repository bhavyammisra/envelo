<script>
  import { settings, set } from './settings.svelte.js';

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
</div>
