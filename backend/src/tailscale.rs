use std::{net::Ipv4Addr, process::Command};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::Value;

use crate::model::is_tailnet_ip;

#[derive(Clone, Debug, Serialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub ipv4: Ipv4Addr,
    pub online: bool,
    pub os: String,
}

fn command(args: &[&str]) -> Result<String> {
    let output = Command::new("tailscale")
        .args(args)
        .output()
        .context("could not run tailscale CLI")?;
    if !output.status.success() {
        bail!(
            "tailscale {} failed: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("tailscale returned non-UTF-8 output")
}

pub fn own_ipv4() -> Result<Ipv4Addr> {
    let output = command(&["ip", "-4"])?;
    let ip: Ipv4Addr = output.trim().parse().context("invalid Tailscale IPv4")?;
    if !is_tailnet_ip(ip) {
        bail!("Tailscale did not return a tailnet IPv4 address");
    }
    Ok(ip)
}

fn node_from_json(value: &Value) -> Option<Node> {
    let id = value.get("ID")?.as_str()?.to_owned();
    let name = value
        .get("DNSName")
        .and_then(Value::as_str)
        .and_then(|dns| dns.split('.').next())
        .filter(|name| !name.is_empty())
        .or_else(|| value.get("HostName").and_then(Value::as_str))?
        .to_owned();
    let ipv4 = value
        .get("TailscaleIPs")?
        .as_array()?
        .iter()
        .filter_map(|entry| entry.as_str()?.parse::<Ipv4Addr>().ok())
        .find(|ip| is_tailnet_ip(*ip))?;
    Some(Node {
        id,
        name,
        ipv4,
        online: value
            .get("Online")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        os: value
            .get("OS")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
    })
}

pub fn nodes() -> Result<Vec<Node>> {
    let value: Value = serde_json::from_str(&command(&["status", "--json"])?)
        .context("invalid tailscale status JSON")?;
    let peers = value
        .get("Peer")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("tailscale status has no Peer object"))?;
    let mut result: Vec<Node> = peers.values().filter_map(node_from_json).collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

pub fn whois_login(ip: Ipv4Addr) -> Result<String> {
    if !is_tailnet_ip(ip) {
        bail!("request did not originate from a Tailscale IPv4 address");
    }
    let value: Value = serde_json::from_str(&command(&["whois", "--json", &ip.to_string()])?)
        .context("invalid tailscale whois JSON")?;
    value
        .get("UserProfile")
        .and_then(|user| user.get("LoginName"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("Tailscale peer has no user login (tagged device?)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_peer() {
        let value = serde_json::json!({
            "ID": "node-id", "HostName": "machine", "OS": "linux", "Online": true,
            "TailscaleIPs": ["100.101.102.103", "fd7a:115c:a1e0::1"]
        });
        let node = node_from_json(&value).unwrap();
        assert_eq!(node.name, "machine");
        assert_eq!(node.ipv4.to_string(), "100.101.102.103");
    }
}
