import { computed, onMounted, onUnmounted, shallowRef } from 'vue'
import { api } from '../api'
import type { Forward, ForwardInput, LiveRule, Node, Status } from '../types'

export function useDashboard() {
  const status = shallowRef<Status | null>(null)
  const nodes = shallowRef<Node[]>([])
  const forwards = shallowRef<Forward[]>([])
  const liveRules = shallowRef<LiveRule[]>([])
  const loading = shallowRef(true)
  const saving = shallowRef(false)
  const error = shallowRef('')
  const notice = shallowRef('')
  const otherRules = computed(() => liveRules.value.filter((rule) => !rule.managed))
  let timer: ReturnType<typeof setInterval> | undefined

  async function refresh(silent = false) {
    if (!silent) loading.value = true
    try {
      const [nextStatus, nextNodes, nextForwards, nextRules] = await Promise.all([
        api.status(), api.nodes(), api.forwards(), api.firewall(),
      ])
      status.value = nextStatus
      nodes.value = nextNodes
      forwards.value = nextForwards
      liveRules.value = nextRules
      error.value = ''
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Unable to load the control plane.'
    } finally {
      loading.value = false
    }
  }

  async function save(input: ForwardInput, id?: string) {
    saving.value = true
    notice.value = ''
    try {
      if (id) await api.update(id, input)
      else await api.create(input)
      notice.value = id ? 'Forward updated.' : 'Forward created.'
      await refresh(true)
      return true
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Could not save the forward.'
      return false
    } finally {
      saving.value = false
    }
  }

  async function remove(id: string) {
    saving.value = true
    notice.value = ''
    try {
      await api.remove(id)
      notice.value = 'Forward removed.'
      await refresh(true)
      return true
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Could not remove the forward.'
      return false
    } finally {
      saving.value = false
    }
  }

  async function toggle(forward: Forward) {
    const { id, label, protocol, public_port, node_id, target_port, enabled } = forward
    return save({ label, protocol, public_port, node_id, target_port, enabled: !enabled }, id)
  }

  onMounted(() => {
    void refresh()
    timer = setInterval(() => void refresh(true), 15_000)
  })
  onUnmounted(() => { if (timer) clearInterval(timer) })

  return {
    status, nodes, forwards, otherRules, loading, saving, error, notice,
    refresh, save, remove, toggle,
  }
}
