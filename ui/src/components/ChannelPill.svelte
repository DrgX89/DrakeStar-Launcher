<script lang="ts">
  import type { Channel } from '../lib/types';
  import { brand } from '../lib/brand';
  let { channel }: { channel: Channel } = $props();
</script>

<img class="pill-img" src={brand.channel(channel)} alt={`canal ${channel}`} />
