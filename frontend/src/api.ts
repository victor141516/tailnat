import type { Forward, ForwardInput, LiveRule, Node, Status } from './types'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api/v1${path}`, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    cache: 'no-store',
  })
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { error?: string } | null
    throw new Error(body?.error ?? `Request failed (${response.status})`)
  }
  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

export const api = {
  status: () => request<Status>('/status'),
  nodes: () => request<Node[]>('/nodes'),
  forwards: () => request<Forward[]>('/forwards'),
  firewall: () => request<LiveRule[]>('/firewall'),
  create: (input: ForwardInput) => request<Forward>('/forwards', {
    method: 'POST', body: JSON.stringify(input),
  }),
  update: (id: string, input: ForwardInput) => request<Forward>(`/forwards/${encodeURIComponent(id)}`, {
    method: 'PUT', body: JSON.stringify(input),
  }),
  remove: (id: string) => request<void>(`/forwards/${encodeURIComponent(id)}`, { method: 'DELETE' }),
}
