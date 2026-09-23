use std::{collections::HashSet, net::Ipv4Addr};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReservedPort {
    pub protocol: Protocol,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Forward {
    pub id: Uuid,
    pub label: String,
    pub protocol: Protocol,
    pub public_port: u16,
    pub node_id: String,
    pub node_name: String,
    pub target_ip: Ipv4Addr,
    pub target_port: u16,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ForwardInput {
    pub label: String,
    pub protocol: Protocol,
    pub public_port: u16,
    pub node_id: String,
    pub target_port: u16,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub listen_port: u16,
    pub public_interface: String,
    #[serde(default = "default_tailnet_interface")]
    pub tailscale_interface: String,
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub reserved_ports: Vec<ReservedPort>,
    #[serde(default)]
    pub forwards: Vec<Forward>,
}

fn default_tailnet_interface() -> String {
    "tailscale0".to_owned()
}

pub fn is_tailnet_ip(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn valid_interface(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 15
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.listen_port == 0 {
            bail!("listen_port must be between 1 and 65535");
        }
        if !valid_interface(&self.public_interface)
            || !valid_interface(&self.tailscale_interface)
            || self.public_interface == self.tailscale_interface
        {
            bail!("invalid or identical network interfaces");
        }
        if self.allowed_users.is_empty() || self.allowed_users.iter().any(|u| u.trim().is_empty()) {
            bail!("allowed_users must contain at least one Tailscale login");
        }

        let reserved: HashSet<_> = self
            .reserved_ports
            .iter()
            .map(|p| (p.protocol, p.port))
            .collect();
        let mut ports = HashSet::new();
        let mut ids = HashSet::new();
        for forward in &self.forwards {
            if !ids.insert(forward.id) {
                bail!("duplicate forward id");
            }
            if forward.label.trim().is_empty() || forward.label.chars().count() > 80 {
                bail!("forward labels must contain 1–80 characters");
            }
            if forward.public_port == 0 || forward.target_port == 0 {
                bail!("ports must be between 1 and 65535");
            }
            if !is_tailnet_ip(forward.target_ip) || forward.node_id.trim().is_empty() {
                bail!("forward destination must be a Tailscale IPv4 peer");
            }
            let key = (forward.protocol, forward.public_port);
            if reserved.contains(&key) {
                bail!("a forward uses a reserved local port");
            }
            if !ports.insert(key) {
                bail!("duplicate public protocol/port");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailnet_range_is_exact() {
        assert!(is_tailnet_ip("100.64.0.1".parse().unwrap()));
        assert!(is_tailnet_ip("100.127.255.254".parse().unwrap()));
        assert!(!is_tailnet_ip("100.128.0.1".parse().unwrap()));
        assert!(!is_tailnet_ip("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn duplicate_ports_are_rejected() {
        let id = Uuid::new_v4();
        let rule = Forward {
            id,
            label: "Web".into(),
            protocol: Protocol::Tcp,
            public_port: 80,
            node_id: "node-a".into(),
            node_name: "node-a".into(),
            target_ip: "100.100.0.1".parse().unwrap(),
            target_port: 80,
            enabled: true,
        };
        let config = Config {
            listen_port: 8787,
            public_interface: "eth0".into(),
            tailscale_interface: "tailscale0".into(),
            allowed_users: vec!["admin@example.com".into()],
            reserved_ports: vec![],
            forwards: vec![
                rule.clone(),
                Forward {
                    id: Uuid::new_v4(),
                    ..rule
                },
            ],
        };
        assert!(config.validate().is_err());
    }
}
