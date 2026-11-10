<script lang="ts">
  import { tick } from "svelte";
  import { api } from "../lib/api";
  import { app } from "../lib/state.svelte";
  import { time, bytes, when } from "../lib/format";
  import StatePill from "../components/StatePill.svelte";
  import type { LogFile, Profile } from "../lib/types";

  let { p, detached = false }: { p: Profile; detached?: boolean } = $props();
  let cmd = $state("");
  let follow = $state(true);
  let box: HTMLDivElement | undefined = $state();
  let hist: string[] = [];
  let hpos = -1;
  let logs = $state<LogFile[]>([]);
  let opened = $state<{ name: string; lines: string[] } | null>(null);

  const lines = $derived(app.consoles[p?.id] ?? []);
  const st = $derived(app.statusOf(p?.id ?? ""));
  const running = $derived(st.state === "running");

  $effect(() => {
    if (p?.id) {
      void app.loadConsole(p.id);
      void loadLogs();
    }
  });

  $effect(() => {
    void lines.length;
    if (follow) void tick().then(() => { if (box) box.scrollTop = box.scrollHeight; });
  });

  async function loadLogs() {
    if (p?.id) logs = (await app.guard(() => api.listLogs(p.id))) ?? [];
  }

  async function send(commandText?: string) {
    const l = (commandText ?? cmd).trim();
    if (!l || !p?.id) return;
    const ok = await app.guard(() => api.serverCommand(p.id, l).then(() => true));
    if (ok) {
      hist.unshift(l);
      hpos = -1;
      if (!commandText) cmd = "";
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === "ArrowUp" && hist.length) {
      e.preventDefault();
      hpos = Math.min(hpos + 1, hist.length - 1);
      cmd = hist[hpos];
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      hpos = Math.max(hpos - 1, -1);
      cmd = hpos < 0 ? "" : hist[hpos];
    }
  }

  async function openLog(f: LogFile) {
    if (!p?.id) return;
    const l = await app.guard(() => api.readLog(p.id, f.source, f.name, 1000));
    if (l) opened = { name: `${f.source}/${f.name}`, lines: l };
  }

  function detachWindow() {
    if (typeof window !== "undefined") {
      window.open(
        "#/forge/console-detached",
        "DrakeStarConsole",
        "width=1100,height=750,menubar=no,toolbar=no,location=no,status=no"
      );
    }
  }
</script>

<div class="blackwall-terminal-container">
  <div class="blackwall-header">
    <div class="row between items-center">
      <div class="cyber-title">
        <span class="glitch-icon">⚡</span>
        <div>
          <h2 class="title-text">DRAKESTAR PROTOCOL // BLACKWALL TERMINAL v2.6</h2>
          <span class="subtext">ARCHITECT: <strong>JACKSTAR</strong> (DrakesCraft Labs) · CORE: PURPUR 26.2 × SLIMEFUN v11</span>
        </div>
      </div>
      <div class="row actions-row">
        {#if !detached}
          <button class="btn cyber-detach" onclick={detachWindow} title="Abrir en ventana externa o segunda pantalla">
            ↗ Desacoplar Consola
          </button>
        {/if}
        <StatePill state={st.state} />
        <button
          class="btn small cyber-btn start"
          onclick={() => p?.id && api.serverStart(p.id).then(() => app.notify("ok", "Arrancando servidor Purpur...")).catch((e) => app.notify("err", String(e.message ?? e)))}
          disabled={st.state === "running" || st.state === "stopping"}
        >
          Iniciar
        </button>
        <button
          class="btn small cyber-btn stop"
          onclick={() => p?.id && app.guard(() => api.serverStop(p.id))}
          disabled={!running}
        >
          Detener
        </button>
      </div>
    </div>

    <!-- Blackwall Telemetry Bar -->
    <div class="telemetry-bar">
      <div class="telemetry-item">
        <span class="lbl">INSTANCIA:</span>
        <span class="val mono">{p?.name ?? "Sin perfil"} ({p?.id ?? "—"})</span>
      </div>
      <div class="telemetry-item">
        <span class="lbl">ASIGNACIÓN RAM:</span>
        <span class="val mono highlight">4096 MB · G1GC HYBRID</span>
      </div>
      <div class="telemetry-item">
        <span class="lbl">LOG BUFFER:</span>
        <span class="val mono">{lines.length} líneas</span>
      </div>
      <div class="telemetry-item">
        <span class="lbl">RED LOCAL:</span>
        <span class="val mono ok">0.0.0.0:25565 (LAN/Online)</span>
      </div>
    </div>

    <!-- Quick Commands Bar -->
    <div class="quick-commands-bar">
      <span class="qc-label">⚡ ACCIONES:</span>
      <div class="qc-buttons">
        <button class="qc-btn" onclick={() => send("tps")} disabled={!running}>/tps</button>
        <button class="qc-btn" onclick={() => send("list")} disabled={!running}>/list</button>
        <button class="qc-btn" onclick={() => send("save-all")} disabled={!running}>/save-all</button>
        <button class="qc-btn" onclick={() => send("op Jack")} disabled={!running}>/op Jack</button>
        <button class="qc-btn" onclick={() => send("gamemode creative")} disabled={!running}>/gm creative</button>
        <button class="qc-btn" onclick={() => send("gamemode survival")} disabled={!running}>/gm survival</button>
        <button class="qc-btn" onclick={() => send("whitelist off")} disabled={!running}>/whitelist off</button>
        <button class="qc-btn" onclick={() => send("mvnets doctor")} disabled={!running}>/mvnets doctor</button>
      </div>
    </div>
  </div>

  <!-- Terminal Output Log -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="console blackwall-box" bind:this={box} role="log" aria-live="off" aria-label="Salida del servidor" tabindex="0">
    {#if lines.length === 0}
      <div class="console-empty">
        <span class="cyber-glitch-text">&gt;&gt; DRAKESTAR PROTOCOL TERMINAL READY &lt;&lt;</span>
        <p class="muted">El núcleo local está en espera. Presiona "Iniciar" o lanza la instancia para conectar el streaming del kernel.</p>
      </div>
    {/if}
    {#each lines as l, i (i)}
      <span class="l {l.stream}">
        <span class="ts">{time(l.ts)}</span>
        {#if l.stream === "stdin"}
          <span class="prompt-prefix">JACKSTAR@SRV:~$ </span>
        {/if}
        {l.line}
      </span>
    {/each}
  </div>

  <!-- CLI Command Input -->
  <form class="cmdline cyber-cmdline" onsubmit={(e) => { e.preventDefault(); void send(); }}>
    <label for="cmd" class="sr-only">Comando para el servidor</label>
    <div class="prompt-box">JACKSTAR@SRV:~$</div>
    <input
      id="cmd"
      bind:value={cmd}
      onkeydown={key}
      placeholder={running ? "Escribe un comando (p. ej. list, say hola, tps) — ↑/↓ para historial" : "El servidor no está en ejecución"}
      disabled={!running}
      autocomplete="off"
      spellcheck="false"
    />
    <button class="btn primary cyber-send" type="submit" disabled={!running || !cmd.trim()}>
      EJECUTAR
    </button>
  </form>

  <div class="row between terminal-footer" style="margin-top:.5rem">
    <div class="row items-center">
      <label style="color:var(--text);margin:0;cursor:pointer"><input type="checkbox" bind:checked={follow} /> Seguir salida</label>
      <button class="btn small" onclick={() => p?.id && app.guard(() => api.consoleClear(p.id).then(() => (app.consoles[p.id] = [])))}>Limpiar vista</button>
    </div>
    <span class="muted mono">Log activo: <code>{st.log_file ?? "—"}</code></span>
  </div>

  {#if !detached}
    <section class="card stack" style="margin-top:1.5rem" aria-labelledby="c-logs">
      <div class="row between">
        <h2 id="c-logs">Archivos de log en disco</h2>
        <button class="btn small" onclick={loadLogs}>Actualizar</button>
      </div>
      {#if logs.length === 0}
        <p class="muted">Aún no hay logs en disco.</p>
      {/if}
      <ul style="margin:0;padding-left:1.1rem">
        {#each logs as f (f.source + f.name)}
          <li>
            <button class="btn small" onclick={() => openLog(f)}>{f.source}/{f.name}</button>
            <span class="muted">{bytes(f.size)} · {when(f.modified)}</span>
          </li>
        {/each}
      </ul>
      {#if opened}
        <h3>{opened.name}</h3>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <pre class="console" tabindex="0">{opened.lines.join("\n")}</pre>
      {/if}
    </section>
  {/if}
</div>

<style>
  .blackwall-terminal-container {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .blackwall-header {
    background: linear-gradient(135deg, rgba(20, 10, 15, 0.95), rgba(10, 15, 25, 0.95));
    border: 1px solid rgba(255, 0, 60, 0.4);
    border-radius: 8px;
    padding: 0.85rem 1.2rem;
    box-shadow: 0 4px 20px rgba(255, 0, 60, 0.15);
  }
  .cyber-title {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .glitch-icon {
    font-size: 1.5rem;
    color: #ff003c;
    text-shadow: 0 0 10px #ff003c, 0 0 20px #00f0ff;
  }
  .title-text {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #f0f6fc;
    font-family: monospace;
  }
  .subtext {
    font-size: 0.75rem;
    color: #8b949e;
    letter-spacing: 0.05em;
  }
  .subtext strong {
    color: #00f0ff;
  }
  .cyber-detach {
    background: rgba(0, 240, 255, 0.1);
    border: 1px solid rgba(0, 240, 255, 0.4);
    color: #00f0ff;
    font-size: 0.8rem;
    font-family: monospace;
    transition: all 0.2s ease;
  }
  .cyber-detach:hover {
    background: rgba(0, 240, 255, 0.25);
    box-shadow: 0 0 12px rgba(0, 240, 255, 0.4);
  }
  .cyber-btn.start {
    background: rgba(34, 197, 94, 0.15);
    border: 1px solid rgba(34, 197, 94, 0.5);
    color: #4ade80;
  }
  .cyber-btn.stop {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.5);
    color: #f87171;
  }
  .telemetry-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 1.25rem;
    margin-top: 0.75rem;
    padding-top: 0.65rem;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 0.8rem;
  }
  .telemetry-item .lbl {
    color: #8b949e;
    font-size: 0.7rem;
    margin-right: 0.35rem;
  }
  .telemetry-item .val.highlight {
    color: #00f0ff;
    font-weight: bold;
  }
  .telemetry-item .val.ok {
    color: #22c55e;
  }
  .quick-commands-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.65rem;
    padding-top: 0.5rem;
    border-top: 1px dashed rgba(255, 0, 60, 0.25);
    overflow-x: auto;
  }
  .qc-label {
    font-size: 0.75rem;
    color: #ff003c;
    font-weight: bold;
    white-space: nowrap;
    font-family: monospace;
  }
  .qc-buttons {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }
  .qc-btn {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #c9d1d9;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    font-family: monospace;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .qc-btn:hover:not(:disabled) {
    background: rgba(0, 240, 255, 0.15);
    border-color: #00f0ff;
    color: #00f0ff;
  }
  .qc-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .blackwall-box {
    background: #05070a !important;
    border: 1px solid rgba(0, 240, 255, 0.3) !important;
    box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.8), 0 0 10px rgba(0, 240, 255, 0.1);
    min-height: 380px;
  }
  .console-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 250px;
    text-align: center;
  }
  .cyber-glitch-text {
    font-family: monospace;
    color: #ff003c;
    font-weight: bold;
    letter-spacing: 0.1em;
    text-shadow: 0 0 8px rgba(255, 0, 60, 0.7);
  }
  .prompt-prefix {
    color: #00f0ff;
    font-weight: bold;
  }
  .cyber-cmdline {
    display: flex;
    align-items: center;
    background: #090d14;
    border: 1px solid rgba(0, 240, 255, 0.4);
    border-radius: 6px;
    padding: 0.25rem 0.5rem;
  }
  .prompt-box {
    font-family: monospace;
    font-size: 0.8rem;
    color: #00f0ff;
    padding: 0 0.5rem;
    white-space: nowrap;
    font-weight: bold;
  }
  .cyber-cmdline input {
    background: transparent !important;
    border: none !important;
    color: #f0f6fc !important;
    font-family: monospace;
    font-size: 0.9rem;
    flex: 1;
    outline: none !important;
    box-shadow: none !important;
  }
  .cyber-send {
    background: linear-gradient(135deg, #ff003c, #990024) !important;
    border: none !important;
    font-family: monospace;
    font-weight: bold;
    letter-spacing: 0.05em;
  }
  .cyber-send:hover:not(:disabled) {
    box-shadow: 0 0 12px rgba(255, 0, 60, 0.5);
  }
  .terminal-footer {
    font-size: 0.8rem;
  }
</style>