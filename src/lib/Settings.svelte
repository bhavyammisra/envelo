<script>
  import { onMount } from 'svelte';
  import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart';
  import { settings, set } from './settings.svelte.js';
  import { vault, setPassword, setQuestion, status as vaultStatus, explain } from './vault.svelte.js';

  const inTauri = '__TAURI_INTERNALS__' in window;
  let startup = $state(false);
  let startupError = $state(null);

  onMount(async () => { if (inTauri) { startup = await isEnabled().catch(() => false); await vaultStatus(); } });

  let editing = $state(null); // 'password' | 'question'
  let a = $state(''), b = $state(''), msg = $state(null);
  const begin = (what) => { editing = what; a = b = ''; msg = null; };

  async function saveVault(e) {
    e.preventDefault();
    msg = null;
    try {
      if (editing === 'password') {
        if (a.length < 6) return (msg = 'Use at least 6 characters.');
        if (a !== b) return (msg = "The two passwords don't match.");
        await setPassword(a);
      } else {
        if (!a.trim() || !b.trim()) return (msg = 'Write a question and an answer.');
        await setQuestion(a, b);
      }
      editing = null;
    } catch (err) { msg = explain(err); }
  }

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

  {#if vault.exists}
    <h2>Locked folder</h2>
    {#if !vault.unlocked}
      <div class="set-row"><div><div class="t">Password and security question</div><div class="d">Unlock the Locked folder first to change these.</div></div></div>
    {:else if editing}
      <form class="set-form" onsubmit={saveVault}>
        {#if editing === 'password'}
          <div class="field"><label for="np">New password</label><input id="np" type="password" autocomplete="new-password" bind:value={a} /></div>
          <div class="field"><label for="np2">Confirm new password</label><input id="np2" type="password" autocomplete="new-password" bind:value={b} /></div>
        {:else}
          <div class="field"><label for="nq">New security question</label><input id="nq" bind:value={a} placeholder="What was the name of her dog?" /></div>
          <div class="field"><label for="na">Answer</label><input id="na" bind:value={b} autocomplete="off" placeholder="Capitals and spaces don't matter" /></div>
        {/if}
        {#if msg}<p class="lock-error" role="alert">{msg}</p>{/if}
        <div class="lock-actions">
          <button class="primary" type="submit" disabled={vault.busy}>{vault.busy ? 'Saving…' : 'Save'}</button>
          <button class="ghost-btn" type="button" onclick={() => (editing = null)}>Cancel</button>
        </div>
      </form>
    {:else}
      <div class="set-row"><div><div class="t">Password</div><div class="d">Opens the Locked folder.</div></div><button class="ghost-btn" onclick={() => begin('password')}>Change</button></div>
      <div class="set-row"><div><div class="t">Security question</div><div class="d">{vault.question}</div></div><button class="ghost-btn" onclick={() => begin('question')}>Change</button></div>
    {/if}
    <div class="set-row"><div><div class="t">Locks again</div><div class="d">After 5 minutes without use, and whenever you close Envelo.</div></div></div>
  {/if}
</div>
