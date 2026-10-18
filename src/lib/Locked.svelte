<script>
  import { Lock, Info } from 'lucide-svelte';
  import { vault, setup, unlock, setPassword, explain } from './vault.svelte.js';

  const questions = [
    'What was the name of your first pet?',
    'What street did you grow up on?',
    'What was the name of your first school?',
    'Write your own question',
  ];

  let mode = $state('password'); // 'password' | 'answer'
  let pw = $state(''), pw2 = $state(''), q = $state(questions[0]), custom = $state(''), answer = $state('');
  let error = $state(null);

  const question = $derived(q === questions.at(-1) ? custom : q);
  // the box to type in gets the cursor whenever it appears (switching modes too)
  const focus = (el) => { requestAnimationFrame(() => el.focus()); };

  async function run(fn) {
    error = null;
    try { await fn(); } catch (e) { error = explain(e); }
  }

  function create(e) {
    e.preventDefault();
    if (pw.length < 6) return (error = 'Use at least 6 characters for the password.');
    if (pw !== pw2) return (error = "The two passwords don't match.");
    if (!question.trim()) return (error = 'Write your security question.');
    if (!answer.trim()) return (error = 'Give an answer to your security question.');
    run(async () => { await setup(pw, question, answer); pw = pw2 = answer = ''; });
  }

  function open(e) {
    e.preventDefault();
    if (mode === 'password') run(async () => { await unlock(pw); pw = ''; });
    else run(async () => { await unlock(answer, true); answer = ''; });
  }

  function reset(e) {
    e.preventDefault();
    if (pw.length < 6) return (error = 'Use at least 6 characters for the password.');
    if (pw !== pw2) return (error = "The two passwords don't match.");
    run(async () => { await setPassword(pw); pw = pw2 = ''; mode = 'password'; });
  }
</script>

{#if !vault.exists}
  <form class="lock" onsubmit={create}>
    <div class="ico"><Lock size={22} strokeWidth={1.5} /></div>
    <h1>Set up your locked folder</h1>
    <p>Notes here are encrypted on this laptop. Open them with your password, or with your security question if you forget it.</p>
    <div class="field"><label for="pw">Password</label><input id="pw" type="password" autocomplete="new-password" bind:value={pw} use:focus /></div>
    <div class="field"><label for="pw2">Confirm password</label><input id="pw2" type="password" autocomplete="new-password" bind:value={pw2} /></div>
    <div class="divider"></div>
    <div class="field"><label for="q">Security question</label>
      <select id="q" bind:value={q}>{#each questions as item}<option>{item}</option>{/each}</select></div>
    {#if q === questions.at(-1)}
      <div class="field"><label for="custom">Your question</label><input id="custom" bind:value={custom} placeholder="What was the name of her dog?" /></div>
    {/if}
    <div class="field"><label for="a">Answer</label><input id="a" bind:value={answer} placeholder="Capitals and spaces don't matter" autocomplete="off" />
      <small>Pick something people who know you couldn't guess.</small></div>
    {#if error}<p class="lock-error" role="alert">{error}</p>{/if}
    <button class="primary" type="submit" disabled={vault.busy}>{vault.busy ? 'Locking…' : 'Lock folder'}</button>
    <div class="warn"><Info size={16} strokeWidth={1.5} /><span>If you forget both the password and the answer, these notes can't be recovered.</span></div>
  </form>
{:else if vault.resetting}
  <form class="lock" onsubmit={reset}>
    <div class="ico"><Lock size={22} strokeWidth={1.5} /></div>
    <h1>Choose a new password</h1>
    <p>Your answer was right and the folder is open. Set a new password for next time.</p>
    <div class="field"><label for="npw">New password</label><input id="npw" type="password" autocomplete="new-password" bind:value={pw} use:focus /></div>
    <div class="field"><label for="npw2">Confirm new password</label><input id="npw2" type="password" autocomplete="new-password" bind:value={pw2} /></div>
    {#if error}<p class="lock-error" role="alert">{error}</p>{/if}
    <button class="primary" type="submit" disabled={vault.busy}>{vault.busy ? 'Saving…' : 'Save password'}</button>
  </form>
{:else}
  <form class="lock" onsubmit={open}>
    <div class="ico"><Lock size={22} strokeWidth={1.5} /></div>
    <h1>Locked</h1>
    {#if mode === 'password'}
      <p>Enter your password to open your locked notes.</p>
      <div class="field"><label for="upw">Password</label><input id="upw" type="password" autocomplete="current-password" bind:value={pw} use:focus /></div>
    {:else}
      <p>{vault.question}</p>
      <div class="field"><label for="ua">Answer</label><input id="ua" bind:value={answer} autocomplete="off" use:focus /></div>
    {/if}
    {#if error}<p class="lock-error" role="alert">{error}</p>{/if}
    <div class="lock-actions">
      <button class="primary" type="submit" disabled={vault.busy}>{vault.busy ? 'Opening…' : 'Unlock'}</button>
      <button class="ghost-btn" type="button" onclick={() => { error = null; mode = mode === 'password' ? 'answer' : 'password'; }}>
        {mode === 'password' ? 'Forgot password?' : 'Use password instead'}
      </button>
    </div>
  </form>
{/if}
