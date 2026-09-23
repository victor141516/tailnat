<script setup lang="ts">
import Button from 'primevue/button'
import Tag from 'primevue/tag'
import type { Status } from '../types'

defineProps<{ status: Status | null; loading: boolean }>()
const emit = defineEmits<{ refresh: [] }>()
</script>

<template>
  <header class="header">
    <div class="brand">
      <div class="brand-mark" aria-hidden="true"><i class="pi pi-share-alt" /></div>
      <div>
        <p class="eyebrow">TAILNET CONTROL PLANE</p>
        <h1>TailNAT</h1>
      </div>
    </div>
    <div class="header-actions">
      <Tag
        v-if="status"
        :severity="status.in_sync ? 'success' : 'warn'"
        :value="status.in_sync ? 'Rules in sync' : 'Rules out of sync'"
        :icon="status.in_sync ? 'pi pi-check-circle' : 'pi pi-exclamation-triangle'"
        class="sync-tag"
      />
      <Button icon="pi pi-refresh" label="Refresh" severity="secondary" outlined :loading="loading" @click="emit('refresh')" />
    </div>
  </header>
</template>

<style scoped>
.header { display: flex; justify-content: space-between; align-items: center; gap: 1.5rem; }
.brand { display: flex; align-items: center; gap: 0.95rem; }
.brand-mark { width: 3.1rem; height: 3.1rem; display: grid; place-items: center; border-radius: 0.95rem; background: linear-gradient(135deg, #64e8b7, #39a9dc); color: #08201d; box-shadow: 0 12px 36px #3bd5b222; }
.brand-mark i { font-size: 1.25rem; }
.eyebrow { margin: 0 0 0.15rem; color: var(--muted); font-size: 0.67rem; font-weight: 800; letter-spacing: 0.19em; }
h1 { margin: 0; font-size: 1.52rem; font-weight: 750; letter-spacing: -0.045em; line-height: 1.1; }
.header-actions { display: flex; align-items: center; gap: 0.8rem; }
.sync-tag { min-height: 2rem; }
@media (max-width: 620px) { .header { align-items: flex-start; flex-direction: column; } .header-actions { width: 100%; justify-content: space-between; } }
</style>
