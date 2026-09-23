# Deployment

This guide targets a Linux gateway with a public IPv4 interface, Tailscale and systemd. Substitute your own interface names, tailnet login and ports. Never copy example identities into production unchanged.

## 1. Prepare and back up

Confirm console or alternate administrative access before changing a gateway firewall. Record the existing rules and services:

```sh
sudo iptables-save > ~/iptables.before-tailnat.v4
sudo ip6tables-save > ~/ip6tables.before-tailnat.v6
sudo systemctl list-unit-files '*forward*'
ip -br address
tailscale status
```

Install `iptables` and enable IPv4 forwarding through your distribution's persistent sysctl mechanism:

```sh
sudo install -d -m 0755 /etc/sysctl.d
printf 'net.ipv4.ip_forward=1\n' | sudo tee /etc/sysctl.d/90-tailnat-forward.conf
sudo sysctl -p /etc/sysctl.d/90-tailnat-forward.conf
```

The destination peer's own firewall and Tailscale access policy must permit the gateway to reach its service ports. For return traffic, TailNAT masquerades matching DNAT connections to the gateway's Tailscale IP.

## 2. Build

On a build host with Node.js 20+ and Rust 1.85+:

```sh
cd frontend
npm ci
npm run build
cd ../backend
cargo test --locked
cargo build --release --locked
```

Build against a libc compatible with the gateway. One option for a Debian 12 x86-64 gateway is building inside the official `rust:1-bookworm` container with the project mounted at `/app`:

```sh
docker run --rm -v "$PWD:/app" -w /app/backend rust:1-bookworm cargo build --release --locked
```

## 3. Install

Copy the release binary, `frontend/dist/` and `deploy/tailnat.service` to the gateway, then install them:

```sh
sudo install -m 0755 backend/target/release/tailnat /usr/local/bin/tailnat
sudo install -d -m 0755 /opt/tailnat/web /etc/tailnat
sudo cp -a frontend/dist/. /opt/tailnat/web/
sudo chown -R root:root /opt/tailnat/web
sudo chmod -R u=rwX,go=rX /opt/tailnat/web
sudo cp deploy/config.example.json /etc/tailnat/config.json
sudo chmod 0600 /etc/tailnat/config.json
sudo install -m 0644 deploy/tailnat.service /etc/systemd/system/tailnat.service
```

Edit `/etc/tailnat/config.json`: set `public_interface`, `tailscale_interface`, `listen_port`, `allowed_users`, `reserved_ports`, and initial `forwards`. `allowed_users` contains exact Tailscale login names (usually account email addresses). A forward needs a valid `node_id` and `target_ip` from `tailscale status --json`; use the UI for new forwards after bootstrap.

Start and inspect:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now tailnat
sudo systemctl status tailnat --no-pager
sudo journalctl -u tailnat -n 100 --no-pager
sudo ss -lntp
```

The listener must be bound to the gateway's `100.64.0.0/10` address, **not** to `0.0.0.0` or its public IP. Open `http://<gateway-tailnet-ip>:<listen_port>/` from an allowed tailnet peer. Restrict that port further using Tailscale grants/ACLs. Do not enable Funnel for it.

## 4. Migrate any old forwarding system

If another service or persistent ruleset already owns public-to-tailnet DNAT rules, do not leave duplicate chains after enabling TailNAT. Import the intended forwards into TailNAT first; verify the new `TAILNAT_PRE`, `TAILNAT_FWD` and `TAILNAT_POST` chains and test each service. Then stop/disable the old service, remove **only its identified jumps and chains**, and save or update your distribution's persistent firewall rules. Do not flush entire built-in chains or overwrite Tailscale's `ts-*` chains.

TailNAT's filter chain drops public-to-tailnet traffic not on its list, even if a stale DNAT rule exists elsewhere. Nonetheless, old NAT rules should be removed to avoid confusing future diagnostics and unexpected behavior if TailNAT is stopped.

## 5. Verify

From a machine **outside the VPS**:

- Test the public ports with the appropriate TCP/UDP client.
- Confirm the control-plane port and administrative SSH are not reachable publicly if that is your intended firewall policy.
- Connect to the dashboard over Tailscale, create/pause a test forward, and confirm that `GET /api/v1/status` reports `in_sync: true`.
- Restart `tailnat` and verify that desired rules are reapplied exactly once.

Use `sudo iptables -t nat -S TAILNAT_PRE` and `sudo iptables -S TAILNAT_FWD` to inspect generated rules. `GET /api/v1/firewall` exposes the observed DNAT inventory to authorized tailnet callers.

## Updates and rollback

To update, build a new binary and frontend, back up the current binary/config, replace both artifacts, and `sudo systemctl restart tailnat`. The service reapplies the saved config.

If an update fails, restore the previous binary/config and restart. A failed firewall update may leave a temporary DROP guard in `FORWARD`; a successful subsequent `tailnat` start removes it. Do not remove the guard until you have inspected and restored a known-good forwarding policy.

Stopping TailNAT does **not** automatically delete kernel rules; this avoids unexpectedly cutting active traffic during a service restart. To uninstall, first plan an alternate policy, disable the service, then remove only `TAILNAT_*` jumps/chains and the associated config/assets. Preserve unrelated iptables and Tailscale rules.
