<script lang="ts">
  import type { Cell } from '../lib/types';
  // Etiquetas honestas: 'planned' en slimefun_gui del cliente Rust = "sin verificar".
  let { cell, unverifiedWord = false }: { cell: Cell; unverifiedWord?: boolean } = $props();
  const text = $derived(
    cell.support === 'yes' ? (cell.caveat?.includes('NO esta verificado') ? 'sí (diseño) · conexión sin verificar' : 'sí')
    : cell.support === 'no' ? 'no'
    : unverifiedWord || cell.caveat?.startsWith('sin verificar') ? 'sin verificar' : 'previsto',
  );
</script>

<span class="badge {cell.support}">{text}</span>
{#if cell.caveat}<div class="hint">{cell.caveat}</div>{/if}
