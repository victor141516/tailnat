import type { Forward, Node } from '../../types'

export interface GraphNode {
  id: string
  name: string
  ip: string
  online: boolean
  x: number
  width: number
  forwards: Forward[]
}

export interface GraphEdge {
  forward: Forward
  path: string
  badgeX: number
  badgeY: number
  color: string
  index: number
}

export interface GraphLayout {
  width: number
  height: number
  gatewayX: number
  nodes: GraphNode[]
  edges: GraphEdge[]
}

const colors = ['#53dfb5', '#42c9ef', '#739dff', '#b796f7', '#f4bd6a']
const sidePadding = 36
const nodeGap = 28

function pointOnCubic(startX: number, startY: number, endX: number, endY: number, t: number) {
  const inverse = 1 - t
  const controlY1 = startY + (endY - startY) * 0.36
  const controlY2 = endY - (endY - startY) * 0.36
  return {
    x: inverse ** 3 * startX + 3 * inverse ** 2 * t * startX + 3 * inverse * t ** 2 * endX + t ** 3 * endX,
    y: inverse ** 3 * startY + 3 * inverse ** 2 * t * controlY1 + 3 * inverse * t ** 2 * controlY2 + t ** 3 * endY,
  }
}

export function buildGraphLayout(forwards: Forward[], peers: Node[], viewportWidth: number): GraphLayout {
  const peerById = new Map(peers.map((peer) => [peer.id, peer]))
  const groups = new Map<string, GraphNode>()

  for (const forward of forwards) {
    const peer = peerById.get(forward.node_id)
    const groupId = `${forward.node_id}/${forward.target_ip}`
    let group = groups.get(groupId)
    if (!group) {
      group = {
        id: groupId,
        name: peer?.name ?? forward.node_name,
        ip: forward.target_ip,
        online: peer?.online ?? false,
        x: 0,
        width: 0,
        forwards: [],
      }
      groups.set(groupId, group)
    }
    group.forwards.push(forward)
  }

  const nodes = [...groups.values()]
  for (const node of nodes) node.width = Math.max(244, 90 + node.forwards.length * 36)
  const totalNodeWidth = nodes.reduce((sum, node) => sum + node.width, 0)
  const requiredWidth = totalNodeWidth + Math.max(0, nodes.length - 1) * nodeGap + sidePadding * 2
  const width = Math.max(720, viewportWidth, requiredWidth)
  const maxPerNode = Math.max(1, ...nodes.map((node) => node.forwards.length))
  const height = Math.max(430, 220 + maxPerNode * 72)
  const gatewayX = width / 2
  const sourceY = 115
  const targetY = height - 98
  let left = (width - (requiredWidth - sidePadding * 2)) / 2
  for (const node of nodes) {
    node.x = left + node.width / 2
    left += node.width + nodeGap
  }

  const edges: GraphEdge[] = []
  let globalIndex = 0
  for (const node of nodes) {
    node.forwards.forEach((forward, nodeIndex) => {
      const startSpacing = Math.min(25, 150 / Math.max(1, forwards.length - 1))
      const startX = gatewayX + (globalIndex - (forwards.length - 1) / 2) * startSpacing
      const endSpacing = Math.min(34, (node.width - 68) / Math.max(1, node.forwards.length - 1))
      const endX = node.x + (nodeIndex - (node.forwards.length - 1) / 2) * endSpacing
      const controlY1 = sourceY + (targetY - sourceY) * 0.36
      const controlY2 = targetY - (targetY - sourceY) * 0.36
      const t = node.forwards.length === 1 ? 0.5 : 0.25 + nodeIndex / (node.forwards.length - 1) * 0.5
      const badge = pointOnCubic(startX, sourceY, endX, targetY, t)
      edges.push({
        forward,
        path: `M ${startX} ${sourceY} C ${startX} ${controlY1}, ${endX} ${controlY2}, ${endX} ${targetY}`,
        badgeX: badge.x,
        badgeY: badge.y,
        color: forward.enabled ? colors[globalIndex % colors.length]! : '#77869b',
        index: globalIndex,
      })
      globalIndex += 1
    })
  }

  return { width, height, gatewayX, nodes, edges }
}
