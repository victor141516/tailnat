<script setup lang="ts">
import Column from 'primevue/column'
import DataTable from 'primevue/datatable'
import type { LiveRule } from '../types'

defineProps<{ rules: LiveRule[] }>()
</script>

<template>
  <section class="observed">
    <div class="observed-heading">
      <div>
        <p class="section-kicker">FIREWALL VISIBILITY</p>
        <h2>Other DNAT rules <span>{{ rules.length }}</span></h2>
      </div>
      <p>Read-only rules outside TailNAT's managed chain.</p>
    </div>
    <DataTable v-if="rules.length" :value="rules" size="small" scrollable>
      <Column field="chain" header="Chain" />
      <Column field="protocol" header="Protocol" />
      <Column field="public_port" header="Port" />
      <Column field="destination" header="Destination" />
    </DataTable>
    <div v-else class="no-other"><i class="pi pi-shield" aria-hidden="true" /> No other DNAT entries observed.</div>
  </section>
</template>

<style scoped>
.observed { min-width: 0; overflow-x: auto; padding: 1.45rem 1.6rem; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); }
.observed-heading { display: flex; justify-content: space-between; align-items: end; gap: 1rem; margin-bottom: 1.2rem; }
.observed-heading p { margin: 0; color: var(--muted); font-size: 0.81rem; }
.section-kicker { color: #70dabb !important; font-size: 0.66rem !important; letter-spacing: 0.16em; font-weight: 800; margin-bottom: 0.35rem !important; }
h2 { margin: 0; font-size: 1.15rem; letter-spacing: -0.02em; }
h2 span { color: var(--muted); font-weight: 500; padding-left: 0.3rem; }
.no-other { display: flex; align-items: center; gap: 0.65rem; color: var(--muted); font-size: 0.85rem; min-height: 2.4rem; }
.no-other i { color: #70dabb; }
@media (max-width: 680px) { .observed-heading { align-items: flex-start; flex-direction: column; } .observed { padding: 1.25rem; } }
</style>
