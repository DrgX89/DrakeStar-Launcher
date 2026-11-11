<script lang="ts">
  import { brand } from '../lib/brand';
  import { app } from '../lib/state.svelte';

  let { 
    show = $bindable(false),
    title = 'Error en ejecución',
    errorLog = '',
    component = 'IntegratedServer / Slimefun'
  }: {
    show?: boolean;
    title?: string;
    errorLog?: string;
    component?: string;
  } = $props();

  function openGithubReport() {
    const repoUrl = 'https://github.com/SlimefunNewHorizons/drakestar-fused/issues/new';
    const issueTitle = `[Bug / Crash Report] ${title}`;
    
    // Recortar log si excede los 2500 caracteres para URLs seguras
    const safeLog = errorLog.length > 2500 ? errorLog.slice(-2500) : errorLog;
    
    const body = `### 🚨 Reporte Automático de Error\n\n` +
      `**Componente:** \`${component}\`\n` +
      `**Versión Launcher:** \`v0.5.0\`\n` +
      `**Fecha:** \`${new Date().toISOString()}\`\n\n` +
      `#### 📜 Log de Error Registrado\n\`\`\`text\n${safeLog || 'No se registraron logs específicos.'}\n\`\`\`\n\n` +
      `#### ℹ️ Pasos para reproducir\n1. Abrir DrakeStar Launcher\n2. Ejecutar instancia\n3. [Describir lo que estabas haciendo]\n\n` +
      `*(Generado automáticamente por DrakeStar Telemetry & SRE)*`;

    const fullUrl = `${repoUrl}?title=${encodeURIComponent(issueTitle)}&body=${encodeURIComponent(body)}&labels=crash-report,community-report`;
    window.open(fullUrl, '_blank', 'noopener,noreferrer');
    show = false;
    app.notify('ok', '📋 Formulario de reporte abierto en GitHub. ¡Gracias por reportar!');
  }

  function copyLogToClipboard() {
    navigator.clipboard.writeText(errorLog);
    app.notify('ok', 'Copiado al portapapeles');
  }
</script>

{#if show}
  <div class="modal-backdrop" onclick={() => (show = false)} onkeydown={(e) => e.key === 'Escape' && (show = false)} role="button" tabindex="0" aria-label="Cerrar modal">
    <div class="modal-card card stack" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
      <header class="row between items-center">
        <div class="row items-center gap-sm">
          <span style="font-size:1.5rem">💥</span>
          <h2 style="margin:0; color:#ef4444; font-size:1.1rem;">REPORTE DE ERROR / CRASH</h2>
        </div>
        <button class="btn icon-btn" onclick={() => (show = false)}>✕</button>
      </header>

      <p class="muted" style="font-size:0.85rem; margin:0.25rem 0;">
        Se ha detectado una falla en el servidor o cliente de Minecraft. Puedes enviar el reporte directamente a GitHub con un solo clic para que el equipo SRE lo analice y genere un fix.
      </p>

      <div class="log-preview-box">
        <div class="row between items-center log-preview-header">
          <span class="mono" style="font-size:0.75rem; color:#9ca3af;">ÚLTIMAS LÍNEAS DEL REGISTRO</span>
          <button class="btn small" onclick={copyLogToClipboard}>Copiar Log</button>
        </div>
        <pre class="log-pre mono">{errorLog || 'Sin detalles adicionales en el búfer.'}</pre>
      </div>

      <footer class="row between items-center" style="margin-top:0.5rem;">
        <button class="btn" onclick={() => (show = false)}>Ignorar / Cerrar</button>
        <button class="btn primary" onclick={openGithubReport} style="background: linear-gradient(135deg, #ef4444, #b91c1c); border:none; font-weight:bold; color:#fff;">
          🐙 ABRIR REPORTE EN GITHUB (1-CLIC)
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    padding: 1rem;
  }

  .modal-card {
    max-width: 620px;
    width: 100%;
    background: #0d1117;
    border: 1px solid rgba(239, 68, 68, 0.4);
    box-shadow: 0 8px 32px rgba(239, 68, 68, 0.2);
    border-radius: 10px;
  }

  .log-preview-box {
    background: #05070a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    overflow: hidden;
  }

  .log-preview-header {
    background: rgba(255, 255, 255, 0.03);
    padding: 0.35rem 0.65rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .log-pre {
    margin: 0;
    padding: 0.65rem;
    font-size: 0.75rem;
    color: #fca5a5;
    max-height: 200px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
