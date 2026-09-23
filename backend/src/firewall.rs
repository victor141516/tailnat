use std::{collections::HashSet, process::Command};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::model::Config;

const PRE: &str = "TAILNAT_PRE";
const POST: &str = "TAILNAT_POST";
const FWD: &str = "TAILNAT_FWD";

#[derive(Clone, Debug, Serialize)]
pub struct LiveRule {
    pub chain: String,
    pub protocol: String,
    pub public_port: u16,
    pub destination: String,
    pub managed: bool,
}

fn call(program: &str, args: &[String]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("could not run {program}"))?;
    if !output.status.success() {
        bail!(
            "{program} failed ({}): {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("firewall command returned non-UTF-8 output")
}

fn ipt(table: &str, args: &[&str]) -> Result<String> {
    let mut full = vec![
        "-w".to_owned(),
        "10".to_owned(),
        "-t".to_owned(),
        table.to_owned(),
    ];
    full.extend(args.iter().map(|s| (*s).to_owned()));
    call("iptables", &full)
}

fn exists(table: &str, args: &[&str]) -> Result<bool> {
    let mut full = vec![
        "-w".to_owned(),
        "10".to_owned(),
        "-t".to_owned(),
        table.to_owned(),
    ];
    full.extend(args.iter().map(|s| (*s).to_owned()));
    let output = Command::new("iptables")
        .args(full)
        .output()
        .context("could not run iptables check")?;
    Ok(output.status.success())
}

fn ensure_chain(table: &str, chain: &str) -> Result<()> {
    if !exists(table, &["-S", chain])? {
        ipt(table, &["-N", chain])?;
    }
    Ok(())
}

fn remove_all_jumps(table: &str, parent: &str, spec: &[&str]) -> Result<()> {
    let mut check = vec!["-C", parent];
    check.extend(spec);
    while exists(table, &check)? {
        let mut delete = vec!["-D", parent];
        delete.extend(spec);
        ipt(table, &delete)?;
    }
    Ok(())
}

fn guard_spec(config: &Config) -> Vec<&str> {
    vec![
        "-i",
        &config.public_interface,
        "-o",
        &config.tailscale_interface,
        "-j",
        "DROP",
    ]
}

/// Rebuilds only TAILNAT_* chains. A temporary FORWARD drop stays in place
/// throughout the update; a failed update is closed rather than permissive.
pub fn apply(config: &Config) -> Result<()> {
    config.validate()?;
    ensure_chain("filter", FWD)?;
    ensure_chain("nat", PRE)?;
    ensure_chain("nat", POST)?;

    let guard = guard_spec(config);
    let mut guard_check = vec!["-C", "FORWARD"];
    guard_check.extend(guard.iter().copied());
    if !exists("filter", &guard_check)? {
        let mut insert = vec!["-I", "FORWARD", "1"];
        insert.extend(guard.iter().copied());
        ipt("filter", &insert)?;
    }

    ipt("filter", &["-F", FWD])?;
    ipt("filter", &["-A", FWD, "-j", "DROP"])?;
    ipt("nat", &["-F", PRE])?;
    ipt("nat", &["-F", POST])?;

    let mut masquerade_keys = HashSet::new();
    for forward in config.forwards.iter().filter(|forward| forward.enabled) {
        let ip = forward.target_ip.to_string();
        let target = forward.target_port.to_string();
        let public = forward.public_port.to_string();
        let destination = format!("{ip}:{target}");
        let protocol = forward.protocol.as_str();

        ipt(
            "nat",
            &[
                "-A",
                PRE,
                "-p",
                protocol,
                "--dport",
                &public,
                "-j",
                "DNAT",
                "--to-destination",
                &destination,
            ],
        )?;
        // Insert before the terminal DROP, not after it.
        ipt(
            "filter",
            &[
                "-I",
                FWD,
                "1",
                "-m",
                "conntrack",
                "--ctstate",
                "DNAT",
                "-d",
                &ip,
                "-p",
                protocol,
                "--dport",
                &target,
                "-j",
                "ACCEPT",
            ],
        )?;
        if masquerade_keys.insert((ip.clone(), protocol, target.clone())) {
            ipt(
                "nat",
                &[
                    "-A",
                    POST,
                    "-m",
                    "conntrack",
                    "--ctstate",
                    "DNAT",
                    "-d",
                    &ip,
                    "-o",
                    &config.tailscale_interface,
                    "-p",
                    protocol,
                    "--dport",
                    &target,
                    "-j",
                    "MASQUERADE",
                ],
            )?;
        }
    }

    let pre_spec = ["-i", config.public_interface.as_str(), "-j", PRE];
    remove_all_jumps("nat", "PREROUTING", &pre_spec)?;
    ipt(
        "nat",
        &[
            "-I",
            "PREROUTING",
            "1",
            "-i",
            &config.public_interface,
            "-j",
            PRE,
        ],
    )?;

    let post_spec = ["-j", POST];
    remove_all_jumps("nat", "POSTROUTING", &post_spec)?;
    ipt("nat", &["-A", "POSTROUTING", "-j", POST])?;

    let forward_spec = [
        "-i",
        config.public_interface.as_str(),
        "-o",
        config.tailscale_interface.as_str(),
        "-j",
        FWD,
    ];
    remove_all_jumps("filter", "FORWARD", &forward_spec)?;
    ipt(
        "filter",
        &[
            "-I",
            "FORWARD",
            "1",
            "-i",
            &config.public_interface,
            "-o",
            &config.tailscale_interface,
            "-j",
            FWD,
        ],
    )?;

    remove_all_jumps("filter", "FORWARD", &guard)?;
    Ok(())
}

pub fn live_rules() -> Result<Vec<LiveRule>> {
    let output = call("iptables-save", &["-t".to_owned(), "nat".to_owned()])?;
    Ok(parse_live_rules(&output))
}

fn parse_live_rules(output: &str) -> Vec<LiveRule> {
    output
        .lines()
        .filter_map(|line| {
            let words: Vec<_> = line.split_whitespace().collect();
            if words.first() != Some(&"-A") || !words.windows(2).any(|w| w == ["-j", "DNAT"]) {
                return None;
            }
            let value = |flag: &str| {
                words
                    .iter()
                    .position(|word| *word == flag)
                    .and_then(|index| words.get(index + 1).copied())
            };
            Some(LiveRule {
                chain: words.get(1)?.to_string(),
                protocol: value("-p")?.to_string(),
                public_port: value("--dport")?.parse().ok()?,
                destination: value("--to-destination")?.to_string(),
                managed: words.get(1) == Some(&PRE),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_managed_and_external_dnat() {
        let rules = parse_live_rules(
            "-A TAILNAT_PRE -p tcp -m tcp --dport 443 -j DNAT --to-destination 100.100.1.2:8443\n\
             -A OTHER -p udp --dport 9987 -j DNAT --to-destination 100.100.1.3:9987\n",
        );
        assert_eq!(rules.len(), 2);
        assert!(rules[0].managed);
        assert_eq!(rules[0].public_port, 443);
        assert!(!rules[1].managed);
    }
}
