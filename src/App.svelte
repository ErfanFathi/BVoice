<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import {
    enable as enableAutostart,
    disable as disableAutostart,
    isEnabled as isAutostartEnabled,
  } from "@tauri-apps/plugin-autostart";

  type Config = {
    model: string;
    arm_threshold_ms: number;
    input_device: string | null;
    hotkey: string;
    beam_size: number;
  };

  type Progress = { model: string; downloaded: number; total: number; pct: number };

  const MODELS = [
    { value: "tiny.en", label: "tiny.en", size: "~75 MB" },
    { value: "base.en", label: "base.en", size: "~142 MB" },
    { value: "small.en", label: "small.en", size: "~466 MB" },
  ];

  let cfg = $state<Config | null>(null);
  let initial = $state<Config | null>(null);
  let devices = $state<string[]>([]);
  let status = $state("");
  let statusKind = $state<"idle" | "saved" | "error">("idle");
  let progress = $state<Progress | null>(null);
  let loading = $state(false);
  let capturing = $state(false);
  let autostart = $state(false);

  onMount(async () => {
    const c = await invoke<Config>("get_config");
    cfg = c;
    initial = { ...c };
    devices = await invoke<string[]>("list_input_devices");
    autostart = await isAutostartEnabled();

    await listen<Progress>("bvoice:download-progress", (e) => {
      progress = e.payload;
    });
    await listen<string>("bvoice:model-ready", (e) => {
      loading = false;
      progress = null;
      setStatus(`Model ready: ${e.payload}`, "saved");
    });
    await listen<string>("bvoice:model-error", (e) => {
      loading = false;
      progress = null;
      setStatus(`Error: ${e.payload}`, "error");
    });
    await listen<string>("bvoice:hotkey-captured", (e) => {
      if (cfg) cfg.hotkey = e.payload;
      capturing = false;
    });
  });

  let dirty = $derived(
    cfg != null && initial != null && JSON.stringify(cfg) !== JSON.stringify(initial)
  );
  let modelChanged = $derived(cfg != null && initial != null && cfg.model !== initial.model);

  function setStatus(msg: string, kind: "idle" | "saved" | "error" = "idle") {
    status = msg;
    statusKind = kind;
  }

  async function save() {
    if (!cfg) return;
    const changingModel = !!modelChanged;
    setStatus(changingModel ? "Preparing new model…" : "Saving…", "idle");
    if (changingModel) {
      loading = true;
      progress = null;
    }
    try {
      await invoke("set_config", { new: cfg });
      initial = { ...cfg };
      if (!changingModel) {
        setStatus("Saved", "saved");
      }
    } catch (e) {
      loading = false;
      progress = null;
      setStatus(`Error: ${e}`, "error");
    }
  }

  async function rebindHotkey() {
    capturing = true;
    setStatus("Press any key to bind…", "idle");
    try {
      await invoke("capture_hotkey");
    } catch (e) {
      capturing = false;
      setStatus(`Error: ${e}`, "error");
    }
  }

  async function toggleAutostart(next: boolean) {
    try {
      if (next) await enableAutostart();
      else await disableAutostart();
      autostart = next;
    } catch (e) {
      setStatus(`Autostart error: ${e}`, "error");
    }
  }

  function mb(bytes: number) {
    return Math.round(bytes / 1_048_576);
  }
</script>

<main class="app">
  <header>
    <div class="brand">
      <img class="logo" src="/icon.png" alt="BVoice" />
      <div>
        <h1>BVoice</h1>
        <p>Local push-to-talk dictation</p>
      </div>
    </div>
  </header>

  {#if !cfg}
    <div class="loading">Loading…</div>
  {:else}
    <section class="card">
      <div class="card-head">
        <h2>Transcription model</h2>
      </div>
      <div class="model-grid">
        {#each MODELS as m}
          <label class="model-option" class:active={cfg.model === m.value}>
            <input
              type="radio"
              name="model"
              value={m.value}
              checked={cfg.model === m.value}
              onchange={() => (cfg!.model = m.value)}
              disabled={loading || capturing}
            />
            <div class="model-info">
              <strong>{m.label}</strong>
              <span class="muted small">{m.size}</span>
            </div>
          </label>
        {/each}
      </div>
      {#if loading && progress}
        <div class="progress">
          <div class="bar" style="width: {progress.pct}%"></div>
          <div class="meta">
            Downloading {progress.model}: {progress.pct}% — {mb(progress.downloaded)}/{mb(progress.total)} MB
          </div>
        </div>
      {:else if loading}
        <div class="progress indeterminate">
          <div class="bar"></div>
          <div class="meta">Loading model…</div>
        </div>
      {/if}
      <div class="field">
        <span class="label">Beam size</span>
        <div class="grow row-right">
          <input
            type="number"
            min="1"
            max="10"
            step="1"
            bind:value={cfg.beam_size}
            disabled={loading || capturing}
          />
          <span class="muted small">{cfg.beam_size <= 1 ? "greedy" : "beam search"}</span>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>Trigger</h2>
        <span class="muted">Hold, speak, release</span>
      </div>
      <div class="field">
        <span class="label">Key</span>
        <div class="grow row-right">
          <kbd class:capturing>{capturing ? "Press a key…" : cfg.hotkey}</kbd>
          <button class="ghost" type="button" onclick={rebindHotkey} disabled={loading || capturing}>Rebind</button>
        </div>
      </div>
      <div class="field">
        <span class="label">Arm threshold</span>
        <div class="grow row-right">
          <input
            type="number"
            min="100"
            max="5000"
            step="50"
            bind:value={cfg.arm_threshold_ms}
            disabled={loading || capturing}
          />
          <span class="muted small">ms</span>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>Audio</h2>
        <span class="muted">Input source</span>
      </div>
      <div class="field">
        <span class="label">Device</span>
        <select
          class="grow"
          bind:value={cfg.input_device}
          disabled={loading || capturing}
        >
          <option value={null}>System default</option>
          {#each devices as d}
            <option value={d}>{d}</option>
          {/each}
        </select>
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h2>General</h2>
      </div>
      <div class="field">
        <span class="label">Start on login</span>
        <div class="grow row-right">
          <button
            class="toggle"
            role="switch"
            aria-checked={autostart}
            aria-label="Start on login"
            onclick={() => toggleAutostart(!autostart)}
            disabled={loading || capturing}
          >
            <span class="thumb" class:on={autostart}></span>
          </button>
        </div>
      </div>
    </section>

    <footer>
      <div class="status status-{statusKind}">{status}</div>
      <button class="primary" onclick={save} disabled={!dirty || loading || capturing}>
        Save changes
      </button>
    </footer>

    <p class="hint">
      Hold <kbd class="inline">{cfg.hotkey}</kbd> for {cfg.arm_threshold_ms} ms to begin recording. Release to transcribe and type at the cursor.
    </p>
  {/if}
</main>

<style>
  :global(:root) {
    --accent: #7552eb;
    --accent-weak: rgba(117, 82, 235, 0.12);
    --bg: #fafafa;
    --surface: #ffffff;
    --border: rgba(15, 15, 15, 0.08);
    --text: #111;
    --muted: rgba(17, 17, 17, 0.6);
    --ok: #16a34a;
    --err: #dc2626;
    --radius: 10px;
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --accent: #967dff;
      --accent-weak: rgba(150, 125, 255, 0.14);
      --bg: #1a1a1c;
      --surface: #232327;
      --border: rgba(255, 255, 255, 0.08);
      --text: #f1f1f2;
      --muted: rgba(241, 241, 242, 0.55);
    }
  }

  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font: 13.5px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif;
  }

  .app {
    max-width: 520px;
    margin: 0 auto;
    padding: 20px 22px 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  header .brand {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  h1 {
    font-size: 20px;
  }
  header p {
    font-size: 13px;
  }
  .logo {
    width: 64px;
    height: 64px;
    border-radius: 14px;
    object-fit: contain;
    display: block;
  }
  h1 {
    margin: 0;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  header p {
    margin: 0;
    color: var(--muted);
  }

  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .card-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
  }
  .card-head h2 {
    margin: 0;
    font-size: 12.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  .grow { flex: 1 1 auto; min-width: 0; }
  .row-right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .label {
    flex: 0 0 120px;
    font-size: 13px;
    color: var(--text);
  }

  .model-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 8px;
  }
  .model-option {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .model-option:hover { border-color: var(--accent); }
  .model-option.active {
    border-color: var(--accent);
    background: var(--accent-weak);
  }
  .model-option input { display: none; }
  .model-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  select, input[type="number"] {
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: inherit;
    font: inherit;
    outline: none;
    transition: border-color 0.12s, box-shadow 0.12s;
  }
  select:focus, input[type="number"]:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-weak);
  }
  input[type="number"] { width: 90px; text-align: right; }

  kbd {
    padding: 3px 9px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface);
    font: 500 12px "SF Mono", Menlo, Consolas, monospace;
    color: var(--text);
  }
  kbd.inline {
    padding: 1px 6px;
    font-size: 11px;
  }
  kbd.capturing {
    border-style: dashed;
    border-color: var(--accent);
    color: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% { opacity: 0.55; }
  }

  button {
    font: inherit;
    padding: 7px 14px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: inherit;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s, transform 0.04s;
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  button.ghost:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    padding: 8px 18px;
    font-weight: 500;
  }
  button.primary:hover:not(:disabled) { filter: brightness(1.05); }
  button.primary:active:not(:disabled) { transform: translateY(1px); }

  .toggle {
    width: 40px;
    height: 22px;
    padding: 2px;
    border-radius: 999px;
    background: var(--border);
    border: none;
    cursor: pointer;
    position: relative;
    transition: background 0.12s;
  }
  .toggle[aria-checked="true"] { background: var(--accent); }
  .toggle .thumb {
    display: block;
    width: 18px;
    height: 18px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.16s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }
  .toggle .thumb.on { transform: translateX(18px); }

  .progress {
    margin-top: 4px;
    position: relative;
    height: 24px;
    border-radius: 6px;
    overflow: hidden;
    background: var(--border);
    border: 1px solid var(--border);
  }
  .progress .bar {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), #a185ff);
    transition: width 0.12s linear;
  }
  .progress.indeterminate .bar {
    width: 30%;
    animation: slide 1.2s ease-in-out infinite;
  }
  .progress .meta {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 11px;
    color: var(--text);
    mix-blend-mode: difference;
  }
  @keyframes slide {
    0% { margin-left: -30%; }
    100% { margin-left: 100%; }
  }

  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-top: 4px;
    justify-content: flex-end;
  }
  .status {
    flex: 1 1 auto;
    font-size: 12px;
    color: var(--muted);
    transition: color 0.12s;
  }
  .status-saved { color: var(--ok); }
  .status-error { color: var(--err); }

  .hint {
    margin: 4px 2px 0;
    font-size: 11.5px;
    color: var(--muted);
    text-align: center;
  }

  .loading {
    padding: 40px 0;
    text-align: center;
    color: var(--muted);
  }
</style>
