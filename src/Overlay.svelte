<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  type State = "idle" | "recording" | "transcribing";
  let state = $state<State>("idle");
  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    unlisten = await listen<string>("bvoice:state", (e) => {
      const v = e.payload;
      if (v === "idle" || v === "recording" || v === "transcribing") {
        state = v;
      }
    });
  });

  onDestroy(() => unlisten?.());

  // The OS-driven window drag swallows the matching mouseup event, leaving
  // the document in a "button held" state that suppresses the next click.
  // Synthesise the mouseup ourselves so subsequent drags start on the first
  // click instead of the second.
  async function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    await getCurrentWindow().startDragging();
    window.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay {state}" onmousedown={onMouseDown}>
  <img class="icon" src="/icon.png" alt="BVoice" draggable="false" />
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    margin: 0;
    padding: 0;
    background: transparent;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
  }

  .overlay {
    position: relative;
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: grab;
  }

  .icon {
    position: relative;
    width: 56px;
    height: 56px;
    object-fit: cover;
    border-radius: 50%;
    z-index: 2;
    pointer-events: none;
    user-select: none;
    -webkit-user-select: none;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.5));
    transition: filter 0.25s ease;
  }
  .overlay.recording {
    --pulse: 230, 60, 60;
  }
  .overlay.transcribing {
    --pulse: 255, 140, 30;
  }

  .overlay.recording .icon,
  .overlay.transcribing .icon {
    filter: drop-shadow(0 0 8px rgba(var(--pulse), 0.7));
  }

  /* Rotating conic-gradient sweep around the icon */
  .overlay.recording::before,
  .overlay.transcribing::before {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    width: 74px;
    height: 74px;
    margin-top: -37px;
    margin-left: -37px;
    border-radius: 50%;
    background: conic-gradient(
      from 0deg,
      transparent 0deg,
      rgba(var(--pulse), 0) 30deg,
      rgba(var(--pulse), 1) 220deg,
      transparent 320deg
    );
    -webkit-mask: radial-gradient(
      circle,
      transparent 32px,
      #000 33px,
      #000 36px,
      transparent 37px
    );
    mask: radial-gradient(
      circle,
      transparent 32px,
      #000 33px,
      #000 36px,
      transparent 37px
    );
    z-index: 1;
    pointer-events: none;
    animation: spin 1.1s linear infinite;
  }

  /* Counter-orbiting dot */
  .overlay.recording::after,
  .overlay.transcribing::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    width: 6px;
    height: 6px;
    margin-top: -3px;
    margin-left: -3px;
    border-radius: 50%;
    background: rgba(var(--pulse), 1);
    box-shadow: 0 0 8px 0 rgba(var(--pulse), 0.8);
    z-index: 1;
    pointer-events: none;
    animation: orbit 1.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes orbit {
    from {
      transform: rotate(0deg) translateX(34px) rotate(0deg);
    }
    to {
      transform: rotate(-360deg) translateX(34px) rotate(360deg);
    }
  }
</style>
