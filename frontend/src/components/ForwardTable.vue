<script setup lang="ts">
import Button from 'primevue/button'
import Column from 'primevue/column'
import DataTable from 'primevue/datatable'
import Tag from 'primevue/tag'
import type { Forward, Node } from '../types'

const props = defineProps<{ forwards: Forward[]; nodes: Node[]; busy: boolean }>()
const emit = defineEmits<{
  edit: [forward: Forward]
  toggle: [forward: Forward]
  remove: [forward: Forward]
}>()

function nodeOnline(forward: Forward) {
  return props.nodes.find((node) => node.id === forward.node_id)?.online ?? false
}
</script>

<template>
  <DataTable :value="forwards" data-key="id" striped-rows scrollable class="forwards-table">
    <template #empty>
      <div class="empty">
        <i class="pi pi-directions" aria-hidden="true" />
        <strong>No forwards yet</strong>
        <span>Add a rule to expose a tailnet service through this server.</span>
      </div>
    </template>
    <Column header="Service" style="min-width: 12rem">
      <template #body="{ data }: { data: Forward }">
        <div class="service-cell">
          <span class="service-name">{{ data.label }}</span>
          <span class="service-id">{{ data.id.slice(0, 8) }}</span>
        </div>
      </template>
    </Column>
    <Column header="Public listener" style="min-width: 10rem">
      <template #body="{ data }: { data: Forward }">
        <div class="endpoint"><span class="protocol">{{ data.protocol.toUpperCase() }}</span><code>:{{ data.public_port }}</code></div>
      </template>
    </Column>
    <Column header="Tailnet destination" style="min-width: 15rem">
      <template #body="{ data }: { data: Forward }">
        <div class="destination">
          <span class="destination-name">{{ data.node_name }}<span :class="['node-dot', { online: nodeOnline(data) }]" :title="nodeOnline(data) ? 'Online' : 'Offline or missing'" /></span>
          <code>{{ data.target_ip }}:{{ data.target_port }}</code>
        </div>
      </template>
    </Column>
    <Column header="State" style="min-width: 7rem">
      <template #body="{ data }: { data: Forward }">
        <Tag :severity="data.enabled ? 'success' : 'secondary'" :value="data.enabled ? 'Active' : 'Paused'" />
      </template>
    </Column>
    <Column header="Actions" style="min-width: 10rem">
      <template #body="{ data }: { data: Forward }">
        <div class="row-actions">
          <Button :icon="data.enabled ? 'pi pi-pause' : 'pi pi-play'" :aria-label="data.enabled ? `Pause ${data.label}` : `Activate ${data.label}`" severity="secondary" text rounded :disabled="busy" @click="emit('toggle', data)" />
          <Button icon="pi pi-pencil" :aria-label="`Edit ${data.label}`" severity="secondary" text rounded :disabled="busy" @click="emit('edit', data)" />
          <Button icon="pi pi-trash" :aria-label="`Delete ${data.label}`" severity="danger" text rounded :disabled="busy" @click="emit('remove', data)" />
        </div>
      </template>
    </Column>
  </DataTable>
</template>

<style scoped>
.service-cell, .destination { display: flex; flex-direction: column; gap: 0.32rem; }
.service-name, .destination-name { color: var(--text-strong); font-weight: 650; }
.service-id, .destination code { color: var(--muted); font-size: 0.78rem; }
.destination-name { display: flex; align-items: center; gap: 0.5rem; }
.node-dot { display: inline-block; width: 0.43rem; height: 0.43rem; border-radius: 50%; background: #66738e; }
.node-dot.online { background: #5ee0a8; box-shadow: 0 0 0 0.2rem #5ee0a81c; }
.endpoint { display: flex; align-items: center; gap: 0.55rem; }
.endpoint code { color: var(--text-strong); font-size: 0.91rem; }
.protocol { min-width: 2.8rem; text-align: center; font-size: 0.67rem; font-weight: 800; letter-spacing: 0.06em; color: #92ddd1; background: #4ac8aa20; border-radius: 0.35rem; padding: 0.24rem 0.32rem; }
.row-actions { display: flex; align-items: center; gap: 0.1rem; }
.empty { min-height: 14rem; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.55rem; color: var(--muted); text-align: center; }
.empty i { font-size: 1.8rem; margin-bottom: 0.35rem; color: #67d8bb; }
.empty strong { color: var(--text-strong); font-size: 1rem; }
.empty span { max-width: 22rem; font-size: 0.84rem; }
</style>
