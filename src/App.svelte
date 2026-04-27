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
    input_device: string | null;
    beam_size: number;
    use_vad: boolean;
    vad_threshold: number;
  };

  type Progress = { model: string; downloaded: number; total: number; pct: number };
  type DeviceInfo = { name: string; description: string };

  type ModelOption = { value: string; label: string; size: string };
  type ModelGroup = { family: string; options: ModelOption[] };

  const MODEL_GROUPS: ModelGroup[] = [
    {
      family: "tiny.en",
      options: [
        { value: "tiny.en", label: "Full precision", size: "~75 MB" },
        { value: "tiny.en-q5_1", label: "Q5_1 quantized", size: "~31 MB" },
        { value: "tiny.en-q8_0", label: "Q8_0 quantized", size: "~42 MB" },
      ],
    },
    {
      family: "base.en",
      options: [
        { value: "base.en", label: "Full precision", size: "~142 MB" },
        { value: "base.en-q5_1", label: "Q5_1 quantized", size: "~57 MB" },
        { value: "base.en-q8_0", label: "Q8_0 quantized", size: "~81 MB" },
      ],
    },
    {
      family: "small.en",
      options: [
        { value: "small.en", label: "Full precision", size: "~466 MB" },
        { value: "small.en-q5_1", label: "Q5_1 quantized", size: "~181 MB" },
        { value: "small.en-q8_0", label: "Q8_0 quantized", size: "~253 MB" },
      ],
    },
  ];

  let cfg = $state<Config | null>(null);
  let initial = $state<Config | null>(null);
  let devices = $state<DeviceInfo[]>([]);
  let status = $state("");
  let statusKind = $state<"idle" | "saved" | "error">("idle");
  let progress = $state<Progress | null>(null);
  let loading = $state(false);
  let autostart = $state(false);

  onMount(async () => {
    const c = await invoke<Config>("get_config");
    cfg = c;
    initial = { ...c };
    devices = await invoke<DeviceInfo[]>("list_input_devices");
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
      <div class="field">
        <span class="label">Model</span>
        <select
          class="grow"
          bind:value={cfg.model}
          disabled={loading}
        >
          {#each MODEL_GROUPS as g}
            <optgroup label={g.family}>
              {#each g.options as o}
                <option value={o.value}>{o.label} — {o.size}</option>
              {/each}
            </optgroup>
          {/each}
        </select>
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
            disabled={loading}
          />
          <span class="muted small">{cfg.beam_size <= 1 ? "greedy" : "beam search"}</span>
        </div>
      </div>
      <div class="field">
        <span class="label">Trim silence</span>
        <div class="grow row-right">
          <button
            class="toggle"
            role="switch"
            aria-checked={cfg.use_vad}
            aria-label="Trim silence with VAD"
            onclick={() => (cfg!.use_vad = !cfg!.use_vad)}
            disabled={loading}
          >
            <span class="thumb" class:on={cfg.use_vad}></span>
          </button>
          <span class="muted small">Silero VAD</span>
        </div>
      </div>
      <div class="field">
        <span class="label">VAD threshold</span>
        <div class="grow row-right">
          <input
            type="number"
            min="0.1"
            max="0.9"
            step="0.05"
            bind:value={cfg.vad_threshold}
            disabled={loading || !cfg.use_vad}
          />
          <span class="muted small">speech probability</span>
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
          disabled={loading}
        >
          <option value={null}>System default</option>
          {#each devices as d}
            <option value={d.name}>{d.description}</option>
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
            disabled={loading}
          >
            <span class="thumb" class:on={autostart}></span>
          </button>
        </div>
      </div>
    </section>

    <footer>
      <div class="status status-{statusKind}">{status}</div>
      <button class="primary" onclick={save} disabled={!dirty || loading}>
        Save changes
      </button>
    </footer>

    <p class="hint">
      Hold <kbd class="inline">Ctrl</kbd> + <kbd class="inline">Win</kbd> to record. Release to transcribe and type at the cursor.
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
