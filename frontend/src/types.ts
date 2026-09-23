export type Protocol = 'tcp' | 'udp'

export interface Node {
  id: string
  name: string
  ipv4: string
  online: boolean
  os: string
}

export interface Forward {
  id: string
  label: string
  protocol: Protocol
  public_port: number
  node_id: string
  node_name: string
  target_ip: string
  target_port: number
  enabled: boolean
}

export type ForwardInput = Omit<Forward, 'id' | 'node_name' | 'target_ip'>

export interface LiveRule {
  chain: string
  protocol: string
  public_port: number
  destination: string
  managed: boolean
}

export interface Status {
  version: string
  tailnet_ip: string
  configured_forwards: number
  enabled_forwards: number
  live_managed_rules: number
  in_sync: boolean
}
