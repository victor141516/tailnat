<script setup lang="ts">
import { computed, nextTick, onUnmounted, shallowRef, useTemplateRef, watch } from 'vue'
import Button from 'primevue/button'
import type { Forward, Node as TailNode } from '../../types'
import { buildGraphLayout } from './layout'

const props = defineProps<{ forwards: Forward[]; nodes: TailNode[]; busy: boolean }>()
const emit = defineEmits<{
  edit: [forward: Forward]
  toggle: [forward: Forward]
  remove: [forward: Forward]
}>()

const viewport = useTemplateRef<HTMLDivElement>('viewport')
const viewportWidth = shallowRef(0)
const openId = shallowRef<string | null>(null)
const layout = computed(() => buildGraphLayout(props.forwards, props.nodes, viewportWidth.value))
let observer: ResizeObserver | undefined

watch(viewport, (element) => {
  observer?.disconnect()
  if (!element) return
  viewportWidth.value = element.clientWidth
  observer = new ResizeObserver(([entry]) => {
    if (entry) viewportWidth.value = entry.contentRect.width
  })
  observer.observe(element)
}, { flush: 'post' })
onUnmounted(() => observer?.disconnect())

watch(viewportWidth, async (width, previousWidth) => {
  if (layout.value.width <= width || (previousWidth > 0 && layout.value.width > previousWidth)) return
  await nextTick()
  if (viewport.value) viewport.value.scrollLeft = (layout.value.width - width) / 2
})

watch(() => props.forwards, (forwards) => {
  if (openId.value && !forwards.some((forward) => forward.id === openId.value)) openId.value = null
})

function closeOnFocusOut(event: FocusEvent, id: string) {
  const marker = event.currentTarget as HTMLElement
  if (!marker.contains(event.relatedTarget as globalThis.Node | null) && openId.value === id) {
    openId.value = null
  }
}

function closeMarker(event: KeyboardEvent) {
  openId.value = null
  ;(event.currentTarget as HTMLElement).querySelector<HTMLButtonElement>('.port-badge')?.focus()
}
</script>

<template>
  <div v-if="!forwards.length" class="graph-empty">
    <i class="pi pi-directions" aria-hidden="true" />
    <strong>No forwards yet</strong>
    <span>Add a rule to expose a tailnet service through this server.</span>
  </div>
  <section v-else class="graph" aria-label="Port forwarding graph">
    <p class="graph-hint">Swipe to explore the forwarding graph <i class="pi pi-arrow-right" aria-hidden="true" /></p>
    <div ref="viewport" class="graph-viewport">
      <div class="graph-canvas" :style="{ width: `${layout.width}px`, height: `${layout.height}px` }" @click="openId = null">
        <svg class="routes" :viewBox="`0 0 ${layout.width} ${layout.height}`" aria-hidden="true">
          <g v-for="edge in layout.edges" :key="edge.forward.id" :class="['route', { selected: openId === edge.forward.id, paused: !edge.forward.enabled }]">
            <path class="route-track" :d="edge.path" :stroke="edge.color" />
            <path class="route-core" :d="edge.path" :stroke="edge.color" />
            <path class="route-motion" :d="edge.path" :stroke="edge.forward.enabled ? '#e6fff8' : '#bdc7d2'" />
          </g>
        </svg>

        <div class="gateway" :style="{ left: `${layout.gatewayX}px` }">
          <span class="gateway-icon"><i class="pi pi-globe" aria-hidden="true" /></span>
          <strong>Public gateway</strong>
          <small>Internet ingress</small>
        </div>

        <div
          v-for="node in layout.nodes"
          :key="node.id"
          class="destination"
          :style="{ left: `${node.x - node.width / 2}px`, top: `${layout.height - 89}px`, width: `${node.width}px` }"
        >
          <span class="destination-icon"><i class="pi pi-server" aria-hidden="true" /></span>
          <span class="destination-copy">
            <strong :title="node.name">{{ node.name }}</strong>
            <code :title="node.ip">{{ node.ip }}</code>
          </span>
          <span :class="['node-status', { online: node.online }]" :title="node.online ? 'Online' : 'Offline'" :aria-label="node.online ? 'Online' : 'Offline'" />
          <small v-if="!node.online" class="offline-label">Offline</small>
        </div>

        <div
          v-for="edge in layout.edges"
          :key="`marker-${edge.forward.id}`"
          :class="['marker', { open: openId === edge.forward.id, paused: !edge.forward.enabled, 'popover-left': edge.badgeX > layout.width * 0.62 }]"
          :style="{ left: `${edge.badgeX}px`, top: `${edge.badgeY}px`, '--route-color': edge.color }"
          @pointerenter="openId = edge.forward.id"
          @pointerleave="openId = openId === edge.forward.id ? null : openId"
          @focusin="openId = edge.forward.id"
          @focusout="closeOnFocusOut($event, edge.forward.id)"
          @keydown.esc.stop.prevent="closeMarker"
          @click.stop
        >
          <button
            type="button"
            class="port-badge"
            :aria-expanded="openId === edge.forward.id"
            :aria-label="`${edge.forward.label}: ${edge.forward.protocol.toUpperCase()} port ${edge.forward.public_port} to ${edge.forward.node_name} ${edge.forward.target_ip}:${edge.forward.target_port}, ${edge.forward.enabled ? 'active' : 'paused'}`"
            @click="openId = edge.forward.id"
          >
            <span>{{ edge.forward.public_port }}</span>
            <small v-if="!edge.forward.enabled">Paused</small>
          </button>
          <div v-if="openId === edge.forward.id" class="port-popover">
            <div class="popover-heading">
              <span>{{ edge.forward.protocol.toUpperCase() }} · {{ edge.forward.public_port }}</span>
              <strong>{{ edge.forward.label }}</strong>
              <small>{{ edge.forward.node_name }} · {{ edge.forward.target_ip }}:{{ edge.forward.target_port }}</small>
            </div>
            <div class="popover-actions">
              <Button :icon="edge.forward.enabled ? 'pi pi-pause' : 'pi pi-play'" :label="edge.forward.enabled ? 'Pause' : 'Activate'" severity="secondary" text size="small" :disabled="busy" @click="emit('toggle', edge.forward)" />
              <Button icon="pi pi-pencil" label="Edit / rename" severity="secondary" text size="small" :disabled="busy" @click="emit('edit', edge.forward)" />
              <Button icon="pi pi-trash" label="Delete" severity="danger" text size="small" :disabled="busy" @click="emit('remove', edge.forward)" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.graph { min-width: 0; border-top: 1px solid var(--border); }
.graph-hint { display: none; margin: 0; padding: 0.7rem 1.35rem 0; color: var(--muted); font-size: 0.75rem; }
.graph-viewport { width: 100%; overflow-x: auto; overflow-y: hidden; }
.graph-canvas { position: relative; margin: 0 auto; background: radial-gradient(ellipse 45% 60% at 50% 30%, #1d375033, transparent 80%); }
.routes { position: absolute; inset: 0; width: 100%; height: 100%; overflow: visible; pointer-events: none; }
.route { fill: none; stroke-linecap: round; }
.route-track { stroke-width: 15; opacity: 0.12; }
.route-core { stroke-width: 8; opacity: 0.65; }
.route-motion { stroke-width: 4; stroke-dasharray: 14 17; opacity: 0.85; animation: flow 1.2s linear infinite; }
.route.paused .route-track { opacity: 0.1; }
.route.paused .route-core { opacity: 0.48; }
.route.paused .route-motion { opacity: 0.4; animation: none; }
.route.selected .route-track { opacity: 0.27; }
.route.selected .route-core { opacity: 0.95; }
@keyframes flow { to { stroke-dashoffset: -31; } }

.gateway { position: absolute; top: 14px; transform: translateX(-50%); width: 220px; min-height: 101px; display: grid; justify-items: center; gap: 0.15rem; padding: 0.72rem 0.85rem; border: 1px solid #4baedc8a; border-radius: 0.8rem; background: #1a293e; box-shadow: 0 12px 36px #0005; text-align: center; }
.gateway-icon { color: #51c9f1; font-size: 1.6rem; line-height: 1.1; }
.gateway strong { color: var(--text-strong); font-size: 0.91rem; }
.gateway small { color: var(--muted); font-size: 0.75rem; }
.destination { position: absolute; display: flex; align-items: center; gap: 0.7rem; height: 70px; padding: 0.7rem 0.85rem; border: 1px solid #416782; border-radius: 0.8rem; background: #1a293b; box-shadow: 0 8px 30px #0003; }
.destination-icon { display: grid; flex: 0 0 2.35rem; place-items: center; width: 2.35rem; height: 2.35rem; border-radius: 0.55rem; background: #42d7b920; color: #5ee5c7; font-size: 1.05rem; }
.destination-copy { display: grid; min-width: 0; gap: 0.22rem; }
.destination-copy strong { overflow: hidden; color: var(--text-strong); font-size: 0.85rem; text-overflow: ellipsis; white-space: nowrap; }
.destination-copy code { overflow: hidden; color: var(--muted); font-size: 0.73rem; text-overflow: ellipsis; white-space: nowrap; }
.node-status { flex: 0 0 0.48rem; width: 0.48rem; height: 0.48rem; margin-left: auto; border-radius: 50%; background: #78869b; }
.node-status.online { background: #54deae; box-shadow: 0 0 0 0.2rem #54deae20; }
.offline-label { color: #c7d0dc; font-size: 0.65rem; }

.marker { position: absolute; z-index: 2; transform: translate(-50%, -50%); }
.marker.open { z-index: 5; }
.port-badge { display: grid; place-content: center; min-width: 3.25rem; height: 3.25rem; padding: 0 0.35rem; border: 2px solid var(--route-color); border-radius: 999px; background: #172437; box-shadow: 0 0 0 0.3rem #101b2ac9, 0 0 1.2rem color-mix(in srgb, var(--route-color) 28%, transparent); color: var(--text-strong); font-size: 0.79rem; font-weight: 800; cursor: pointer; transition: transform 160ms ease, box-shadow 160ms ease; }
.port-badge small { color: #c7d0dc; font-size: 0.53rem; font-weight: 600; line-height: 1; }
.marker.open .port-badge, .marker:hover .port-badge, .marker:focus-within .port-badge { transform: scale(1.1); }
.marker.paused .port-badge { background: #212a38; }
.port-popover { position: absolute; top: 50%; left: calc(100% + 0.85rem); width: 13.5rem; transform: translateY(-50%); padding: 0.7rem; border: 1px solid #426477; border-radius: 0.8rem; background: #142031; box-shadow: 0 18px 40px #0009; }
.popover-left .port-popover { right: calc(100% + 0.85rem); left: auto; }
.port-popover::before { position: absolute; top: 0; left: -0.85rem; width: 0.85rem; height: 100%; content: ''; }
.popover-left .port-popover::before { right: -0.85rem; left: auto; }
.popover-heading { display: grid; gap: 0.18rem; padding: 0.2rem 0.35rem 0.55rem; border-bottom: 1px solid var(--border); }
.popover-heading span { color: var(--route-color); font-size: 0.75rem; font-weight: 800; }
.popover-heading strong { overflow: hidden; color: var(--text-strong); font-size: 0.83rem; text-overflow: ellipsis; white-space: nowrap; }
.popover-heading small { overflow: hidden; color: var(--muted); font-size: 0.7rem; text-overflow: ellipsis; white-space: nowrap; }
.popover-actions { display: grid; padding-top: 0.4rem; }
.popover-actions :deep(.p-button) { justify-content: flex-start; min-height: 2.1rem; padding: 0.3rem 0.45rem; font-size: 0.78rem; }
.graph-empty { min-height: 15rem; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.55rem; border-top: 1px solid var(--border); color: var(--muted); text-align: center; }
.graph-empty i { margin-bottom: 0.35rem; color: #67d8bb; font-size: 1.8rem; }
.graph-empty strong { color: var(--text-strong); font-size: 1rem; }
.graph-empty span { max-width: 22rem; font-size: 0.84rem; }
@media (max-width: 760px) { .graph-hint { display: block; } }
@media (prefers-reduced-motion: reduce) { .route-motion { animation: none; } .port-badge { transition: none; } }
</style>
