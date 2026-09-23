# HTTP API

All endpoints are under `/api/v1` and return JSON unless noted. The service listens on the gateway's Tailscale IPv4 address only. Requests must originate from a Tailscale peer whose WhoIs `UserProfile.LoginName` appears in `allowed_users`. There is no separate bearer token. Browser writes are same-origin only; no CORS access is granted.

For command-line integration, connect from an authorized Tailscale node:

```sh
curl http://gateway.tailnet.example:8787/api/v1/forwards
```

The hostname is illustrative; use your gateway's actual MagicDNS name or Tailscale IP. Make requests from a tailnet device, not from the public Internet. Send `Content-Type: application/json` for write requests.

## Endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/status` | Version, control-plane IP, counts and managed-rule drift indicator. |
| `GET` | `/nodes` | Tailnet peers with ID, name, IPv4, OS and online flag. |
| `GET` | `/forwards` | Configured forwards, including paused entries. |
| `POST` | `/forwards` | Create a forward. Returns `201` and the created entry. |
| `PUT` | `/forwards/{id}` | Replace all editable fields of one forward. Returns the updated entry. |
| `DELETE` | `/forwards/{id}` | Remove a forward. Returns `204` with no body. |
| `GET` | `/firewall` | DNAT entries observed in the live IPv4 NAT table, including other chains. |

`POST` and `PUT` require the same complete input object:

```json
{
  "label": "Example web service",
  "protocol": "tcp",
  "public_port": 8443,
  "node_id": "example-tailscale-node-id",
  "target_port": 443,
  "enabled": true
}
```

`node_id` must be one of the IDs from `GET /nodes`. TailNAT resolves its current Tailscale IPv4 itself, rather than accepting an arbitrary IP from the caller. The `label` has 1–80 characters. Ports are 1–65535. A `(protocol, public_port)` pair can appear only once, even if paused, and cannot use a reserved local port. `enabled: false` stores the entry but installs no DNAT rule.

A forward returned by `GET /forwards` adds `id`, `node_name` and `target_ip`:

```json
{
  "id": "c9884cc9-e425-43c9-9996-b21f1d1520fb",
  "label": "Example web service",
  "protocol": "tcp",
  "public_port": 8443,
  "node_id": "example-tailscale-node-id",
  "node_name": "example-node",
  "target_ip": "100.101.102.103",
  "target_port": 443,
  "enabled": true
}
```

To pause an entry, `PUT` the same editable fields with `enabled: false`. `PUT` is a full replacement, not a JSON Patch operation.

`GET /status` includes `in_sync`, which checks whether the live managed DNAT entries match the enabled desired forwards. This does not inspect external chains, end-to-end service health, or return-path connectivity.

`GET /firewall` returns entries such as:

```json
[
  {
    "chain": "TAILNAT_PRE",
    "protocol": "tcp",
    "public_port": 8443,
    "destination": "100.101.102.103:443",
    "managed": true
  }
]
```

An entry with `managed: false` is an observed DNAT command outside TailNAT's chain. It is read-only and may be unreachable, unanchored, shadowed or filtered elsewhere. TailNAT never edits those entries.

## Errors

Errors have the shape `{ "error": "human-readable message" }`.

| Status | Meaning |
|---|---|
| `400` | Invalid rule, duplicate/reserved port, or unknown peer. |
| `403` | Not a Tailscale peer, not on the allowlist, or a cross-origin browser write. |
| `404` | Forward ID not found. |
| `422` | JSON body could not be parsed. |
| `500` | Tailscale, filesystem or firewall operation failed. The daemon attempts to restore the previous rules; on unrecoverable firewall errors its temporary guard remains fail-closed. |

Writes are serialized. A successful response means the desired state was applied to iptables and saved atomically to disk. The service reapplies it on startup.
