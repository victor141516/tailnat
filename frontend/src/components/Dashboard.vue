<script setup lang="ts">
import { shallowRef } from 'vue'
import Button from 'primevue/button'
import Dialog from 'primevue/dialog'
import Message from 'primevue/message'
import StatusHeader from './StatusHeader.vue'
import ForwardTable from './ForwardTable.vue'
import ForwardDialog from './ForwardDialog.vue'
import ObservedRules from './ObservedRules.vue'
import { useDashboard } from '../composables/useDashboard'
import type { Forward, ForwardInput } from '../types'

const { status, nodes, forwards, otherRules, loading, saving, error, notice, refresh, save, remove, toggle } = useDashboard()
const editing = shallowRef<Forward | null>(null)
const dialogVisible = shallowRef(false)
const pendingDelete = shallowRef<Forward | null>(null)

function openCreate() {
  editing.value = null
  dialogVisible.value = true
}

function openEdit(forward: Forward) {
  editing.value = forward
  dialogVisible.value = true
}

async function submit(input: ForwardInput, id?: string) {
  if (await save(input, id)) dialogVisible.value = false
}

async function confirmDelete() {
  if (!pendingDelete.value) return
  if (await remove(pendingDelete.value.id)) pendingDelete.value = null
}
</script>

<template>
  <div class="app-shell">
    <StatusHeader :status="status" :loading="loading" @refresh="refresh()" />

    <main class="main">
      <section class="intro">
        <div>
          <p class="section-kicker">PUBLIC EDGE → PRIVATE TAILNET</p>
          <h2>Port forwarding, under control.</h2>
          <p class="intro-description">Route only the ports you choose to services on your Tailscale nodes. Everything else stays off the forwarding map.</p>
        </div>
        <div class="edge-address" v-if="status">
          <span>CONTROL PLANE</span>
          <strong><i class="pi pi-lock" aria-hidden="true" /> {{ status.tailnet_ip }}</strong>
          <small>Tailnet access only</small>
        </div>
      </section>

      <div v-if="error" class="feedback" role="alert"><Message severity="error" :closable="false">{{ error }}</Message></div>
      <div v-else-if="notice" class="feedback" role="status"><Message severity="success" :closable="false">{{ notice }}</Message></div>

      <section class="forwards-card">
        <div class="card-heading">
          <div>
            <p class="section-kicker">MANAGED RULES</p>
            <h2>Forwards <span>{{ forwards.length }}</span></h2>
            <p>Choose a public port, protocol and destination node.</p>
          </div>
          <Button icon="pi pi-plus" label="Add forward" :disabled="saving" @click="openCreate" />
        </div>
        <p class="table-hint">Swipe across the table for destinations and actions <i class="pi pi-arrow-right" aria-hidden="true" /></p>
        <div class="table-wrap">
          <ForwardTable :forwards="forwards" :nodes="nodes" :busy="saving" @edit="openEdit" @toggle="toggle" @remove="pendingDelete = $event" />
        </div>
        <div class="card-footer">
          <span><i class="pi pi-circle-fill" aria-hidden="true" /> {{ status?.enabled_forwards ?? 0 }} active</span>
          <span>IPv4 forwarding · Tailnet destinations</span>
        </div>
      </section>

      <ObservedRules :rules="otherRules" />
      <footer class="page-footer"><span>TailNAT {{ status?.version ?? '' }}</span><span>Changes are applied immediately and restored when the service starts.</span></footer>
    </main>

    <ForwardDialog v-model="dialogVisible" :forward="editing" :nodes="nodes" :saving="saving" @save="submit" />
    <Dialog :visible="pendingDelete !== null" modal header="Delete forward?" :style="{ width: 'min(92vw, 28rem)' }" :draggable="false" @update:visible="pendingDelete = null">
      <p class="confirm-copy">{{ pendingDelete?.label }} will stop forwarding traffic immediately. This action does not change the destination node.</p>
      <div class="confirm-actions">
        <Button label="Cancel" severity="secondary" text @click="pendingDelete = null" />
        <Button label="Delete forward" icon="pi pi-trash" severity="danger" :loading="saving" @click="confirmDelete" />
      </div>
    </Dialog>
  </div>
</template>

<style scoped>
.app-shell { min-height: 100vh; max-width: 1440px; margin: 0 auto; padding: 2rem clamp(1rem, 4vw, 4rem) 2.5rem; }
.main { display: grid; min-width: 0; gap: 1.5rem; margin-top: 3.4rem; }
.intro { display: flex; min-width: 0; justify-content: space-between; align-items: flex-end; gap: 2rem; padding: 0 0 0.65rem; }
.section-kicker { margin: 0 0 0.55rem; color: #70dabb; font-size: 0.68rem; font-weight: 800; letter-spacing: 0.17em; }
.intro h2 { margin: 0; color: var(--text-strong); font-size: clamp(1.9rem, 3.5vw, 3.1rem); letter-spacing: -0.055em; line-height: 1.08; }
.intro-description { max-width: 39rem; margin: 0.95rem 0 0; color: var(--muted); line-height: 1.58; font-size: 0.94rem; }
.edge-address { min-width: 12.5rem; display: grid; gap: 0.4rem; padding: 1rem 1.2rem; border: 1px solid #4ad4aa3c; border-radius: 0.9rem; background: #2bc9a00c; }
.edge-address span { color: #74ddbe; font-size: 0.63rem; letter-spacing: 0.16em; font-weight: 800; }
.edge-address strong { font: 600 1rem ui-monospace, SFMono-Regular, Consolas, monospace; color: var(--text-strong); }
.edge-address strong i { color: #70dabb; font-size: 0.9rem; margin-right: 0.2rem; }
.edge-address small { color: var(--muted); font-size: 0.73rem; }
.feedback :deep(.p-message) { margin: 0; }
.forwards-card { min-width: 0; overflow: hidden; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); box-shadow: 0 24px 60px #00000013; }
.card-heading { display: flex; justify-content: space-between; align-items: center; gap: 1rem; padding: 1.55rem 1.6rem 1.25rem; }
.card-heading h2 { margin: 0; font-size: 1.34rem; letter-spacing: -0.03em; }
.card-heading h2 span { color: var(--muted); font-weight: 500; padding-left: 0.25rem; }
.card-heading p:last-child { margin: 0.45rem 0 0; color: var(--muted); font-size: 0.83rem; }
.table-wrap { overflow-x: auto; }
.table-hint { display: none; margin: 0; padding: 0 1.3rem 0.8rem; color: var(--muted); font-size: 0.74rem; }
.card-footer { display: flex; justify-content: space-between; gap: 1rem; padding: 0.9rem 1.6rem; border-top: 1px solid var(--border); color: var(--muted); font-size: 0.78rem; }
.card-footer i { color: #59dbac; font-size: 0.43rem; vertical-align: middle; margin-right: 0.32rem; }
.page-footer { display: flex; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; color: #78869e; font-size: 0.73rem; padding: 0.4rem 0.1rem 0; }
.confirm-copy { color: var(--muted); line-height: 1.55; margin: 0.4rem 0 1.5rem; }
.confirm-actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
@media (max-width: 760px) { .main { margin-top: 2.7rem; } .intro { flex-direction: column; align-items: stretch; } .edge-address { width: max-content; } }
@media (max-width: 580px) { .app-shell { padding-top: 1.25rem; } .card-heading { align-items: flex-start; flex-direction: column; padding: 1.3rem; } .card-heading :deep(.p-button) { width: 100%; } .table-hint { display: block; } .card-footer { padding-inline: 1.3rem; } .page-footer { flex-direction: column; } }
</style>
