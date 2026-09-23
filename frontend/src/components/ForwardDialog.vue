<script setup lang="ts">
import { computed, reactive, shallowRef, watch } from 'vue'
import Button from 'primevue/button'
import Dialog from 'primevue/dialog'
import InputNumber from 'primevue/inputnumber'
import InputText from 'primevue/inputtext'
import Select from 'primevue/select'
import SelectButton from 'primevue/selectbutton'
import type { Forward, ForwardInput, Node, Protocol } from '../types'

const props = defineProps<{ forward: Forward | null; nodes: Node[]; saving: boolean }>()
const emit = defineEmits<{ save: [input: ForwardInput, id?: string] }>()
const visible = defineModel<boolean>({ required: true })
const error = shallowRef('')
const protocolOptions: Protocol[] = ['tcp', 'udp']
const form = reactive<{
  label: string; protocol: Protocol; public_port: number | null;
  node_id: string | null; target_port: number | null; enabled: boolean
}>({ label: '', protocol: 'tcp', public_port: null, node_id: null, target_port: null, enabled: true })
const title = computed(() => props.forward ? 'Edit forward' : 'New forward')
const nodeOptions = computed(() => props.nodes.map((node) => ({ ...node, display: `${node.name}${node.online ? '' : ' · offline'}` })))

watch([() => props.forward, visible], () => {
  if (!visible.value) return
  error.value = ''
  Object.assign(form, props.forward ? {
    label: props.forward.label,
    protocol: props.forward.protocol,
    public_port: props.forward.public_port,
    node_id: props.forward.node_id,
    target_port: props.forward.target_port,
    enabled: props.forward.enabled,
  } : { label: '', protocol: 'tcp', public_port: null, node_id: null, target_port: null, enabled: true })
}, { immediate: true })

function submit() {
  if (!form.label.trim() || !form.node_id || !form.public_port || !form.target_port) {
    error.value = 'Complete every field before saving.'
    return
  }
  if (form.public_port < 1 || form.public_port > 65535 || form.target_port < 1 || form.target_port > 65535) {
    error.value = 'Ports must be between 1 and 65535.'
    return
  }
  error.value = ''
  emit('save', {
    label: form.label.trim(), protocol: form.protocol, public_port: form.public_port,
    node_id: form.node_id, target_port: form.target_port, enabled: form.enabled,
  }, props.forward?.id)
}
</script>

<template>
  <Dialog v-model:visible="visible" modal :header="title" :style="{ width: 'min(92vw, 34rem)' }" :draggable="false">
    <form class="forward-form" @submit.prevent="submit">
      <p class="form-intro">Traffic arriving at the public port will be sent to the selected Tailscale node.</p>
      <label class="field">
        <span>Service name</span>
        <InputText v-model="form.label" maxlength="80" placeholder="e.g. Web dashboard" autofocus fluid />
      </label>
      <div class="field-row">
        <div class="field">
          <label for="protocol">Protocol</label>
          <SelectButton id="protocol" v-model="form.protocol" :options="protocolOptions" :allow-empty="false" aria-label="Protocol" />
        </div>
        <label class="field">
          <span>Public port</span>
          <InputNumber v-model="form.public_port" :use-grouping="false" :min="1" :max="65535" placeholder="443" fluid />
        </label>
      </div>
      <label class="field">
        <span>Tailscale node</span>
        <Select v-model="form.node_id" :options="nodeOptions" option-label="display" option-value="id" filter placeholder="Choose a node" fluid />
      </label>
      <label class="field">
        <span>Destination port</span>
        <InputNumber v-model="form.target_port" :use-grouping="false" :min="1" :max="65535" placeholder="443" fluid />
      </label>
      <label class="enabled-line">
        <input v-model="form.enabled" type="checkbox" />
        <span>Enable immediately</span>
      </label>
      <p v-if="error" class="form-error" role="alert">{{ error }}</p>
      <div class="form-actions">
        <Button label="Cancel" severity="secondary" text type="button" @click="visible = false" />
        <Button :label="props.forward ? 'Save changes' : 'Create forward'" icon="pi pi-check" type="submit" :loading="saving" />
      </div>
    </form>
  </Dialog>
</template>

<style scoped>
.forward-form { display: grid; gap: 1.15rem; padding-top: 0.2rem; }
.form-intro { margin: 0 0 0.2rem; color: var(--muted); font-size: 0.88rem; line-height: 1.5; }
.field { display: grid; gap: 0.42rem; min-width: 0; color: var(--text-strong); font-size: 0.83rem; font-weight: 600; }
.field-row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.enabled-line { display: flex; align-items: center; gap: 0.65rem; font-size: 0.84rem; color: var(--text-strong); cursor: pointer; }
.enabled-line input { width: 1rem; height: 1rem; accent-color: #54d4b0; }
.form-error { margin: 0; color: #ffabae; font-size: 0.82rem; }
.form-actions { display: flex; justify-content: flex-end; gap: 0.5rem; padding-top: 0.35rem; }
@media (max-width: 480px) { .field-row { grid-template-columns: 1fr; } }
</style>
