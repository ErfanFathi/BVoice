<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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
</script>

<div class="overlay {state}" data-tauri-drag-region>
  <div class="ring"></div>
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
  .overlay:active {
    cursor: grabbing;
  }

  .icon {
    position: relative;
    width: 56px;
    height: 56px;
    object-fit: contain;
    z-index: 2;
    pointer-events: none;
    user-select: none;
    -webkit-user-select: none;
    transition: opacity 0.2s ease;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.5));
  }

  .overlay.idle .icon {
    opacity: 0.55;
  }
  .overlay.recording .icon,
  .overlay.transcribing .icon {
    opacity: 1;
  }

  /* Pulse rings — recording */
  .overlay.recording::before,
  .overlay.recording::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    width: 56px;
    height: 56px;
    margin-top: -28px;
    margin-left: -28px;
    border-radius: 50%;
    box-shadow: 0 0 0 3px rgba(230, 60, 60, 0.85);
    background: rgba(230, 60, 60, 0.15);
    animation: pulse-ring 1.6s ease-out infinite;
    z-index: 1;
    pointer-events: none;
  }
  .overlay.recording::after {
    animation-delay: 0.8s;
  }

  @keyframes pulse-ring {
    0% {
      transform: scale(0.9);
      opacity: 0.85;
    }
    80% {
      opacity: 0;
    }
    100% {
      transform: scale(1.65);
      opacity: 0;
    }
  }

  /* Spinner ring — transcribing */
  .ring {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 76px;
    height: 76px;
    margin-top: -38px;
    margin-left: -38px;
    border-radius: 50%;
    border: 4px solid rgba(240, 180, 40, 0.18);
    border-top-color: rgba(240, 180, 40, 1);
    box-sizing: border-box;
    z-index: 1;
    pointer-events: none;
    opacity: 0;
    animation: spin 1s linear infinite;
    transition: opacity 0.2s ease;
  }
  .overlay.transcribing .ring {
    opacity: 1;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
