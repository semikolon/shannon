//! nftables/iptables adapter for firewall management

use anyhow::{Context, Result};
use serde::Serialize;

use crate::location::{execute_shell, read_file, write_file};

const IPTABLES_RULES: &str = "/etc/iptables/rules.v4";

#[derive(Debug, Serialize, Clone)]
pub struct PortForward {
    pub external_port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
    pub protocol: String,
    pub comment: Option<String>,
}

pub struct NftablesAdapter;

impl NftablesAdapter {
    pub fn new() -> Self {
        Self
    }

    /// List current port forwarding rules
    pub fn list_port_forwards(&self) -> Result<Vec<PortForward>> {
        let mut forwards = Vec::new();

        // Parse iptables-save output to find DNAT rules
        let output = execute_shell("iptables-save 2>/dev/null | grep DNAT")?;
        let rules = String::from_utf8_lossy(&output.stdout);

        for line in rules.lines() {
            if let Some(forward) = self.parse_dnat_rule(line) {
                forwards.push(forward);
            }
        }

        Ok(forwards)
    }

    fn parse_dnat_rule(&self, line: &str) -> Option<PortForward> {
        // Example: -A PREROUTING -p tcp --dport 8080 -j DNAT --to-destination 192.168.4.84:80
        let parts: Vec<&str> = line.split_whitespace().collect();

        let mut protocol = "tcp".to_string();
        let mut external_port: Option<u16> = None;
        let mut internal_ip = String::new();
        let mut internal_port: u16 = 0;
        let mut comment = None;

        let mut i = 0;
        while i < parts.len() {
            match parts[i] {
                "-p" => {
                    if i + 1 < parts.len() {
                        protocol = parts[i + 1].to_string();
                    }
                }
                "--dport" => {
                    if i + 1 < parts.len() {
                        external_port = parts[i + 1].parse().ok();
                    }
                }
                "--to-destination" => {
                    if i + 1 < parts.len() {
                        let dest = parts[i + 1];
                        let dest_parts: Vec<&str> = dest.split(':').collect();
                        if dest_parts.len() >= 1 {
                            internal_ip = dest_parts[0].to_string();
                            internal_port = dest_parts
                                .get(1)
                                .and_then(|p| p.parse().ok())
                                .unwrap_or(0);
                        }
                    }
                }
                "--comment" => {
                    if i + 1 < parts.len() {
                        comment = Some(parts[i + 1].trim_matches('"').to_string());
                    }
                }
                _ => {}
            }
            i += 1;
        }

        external_port.map(|ext| PortForward {
            external_port: ext,
            internal_ip,
            internal_port: if internal_port > 0 {
                internal_port
            } else {
                ext
            },
            protocol,
            comment,
        })
    }

    /// Add a port forwarding rule
    pub fn add_port_forward(&self, rule: &PortForward) -> Result<()> {
        // Add DNAT rule in nat PREROUTING
        let dnat_cmd = format!(
            "iptables -t nat -A PREROUTING -p {} --dport {} -j DNAT --to-destination {}:{}",
            rule.protocol, rule.external_port, rule.internal_ip, rule.internal_port
        );
        execute_shell(&dnat_cmd)?;

        // Add FORWARD rule to allow the traffic
        let forward_cmd = format!(
            "iptables -A FORWARD -p {} -d {} --dport {} -j ACCEPT",
            rule.protocol, rule.internal_ip, rule.internal_port
        );
        execute_shell(&forward_cmd)?;

        // Persist rules
        self.persist()?;

        Ok(())
    }

    /// Remove a port forwarding rule
    pub fn remove_port_forward(&self, external_port: u16) -> Result<()> {
        // Find and delete the rule by line number
        // First, list rules with line numbers
        let output = execute_shell(&format!(
            "iptables -t nat -L PREROUTING --line-numbers -n | grep 'dpt:{}'",
            external_port
        ))?;

        let rules = String::from_utf8_lossy(&output.stdout);
        for line in rules.lines().rev() {
            // Parse line number (first field)
            if let Some(num_str) = line.split_whitespace().next() {
                if let Ok(num) = num_str.parse::<u32>() {
                    execute_shell(&format!("iptables -t nat -D PREROUTING {}", num))?;
                }
            }
        }

        // Also remove FORWARD rules
        let output = execute_shell(&format!(
            "iptables -L FORWARD --line-numbers -n | grep 'dpt:{}'",
            external_port
        ))?;

        let rules = String::from_utf8_lossy(&output.stdout);
        for line in rules.lines().rev() {
            if let Some(num_str) = line.split_whitespace().next() {
                if let Ok(num) = num_str.parse::<u32>() {
                    execute_shell(&format!("iptables -D FORWARD {}", num))?;
                }
            }
        }

        self.persist()?;
        Ok(())
    }

    /// Block an IP address
    pub fn block_ip(&self, target: &str) -> Result<()> {
        execute_shell(&format!(
            "iptables -I INPUT -s {} -j DROP && iptables -I FORWARD -s {} -j DROP",
            target, target
        ))?;
        self.persist()?;
        Ok(())
    }

    /// Unblock an IP address
    pub fn unblock_ip(&self, target: &str) -> Result<()> {
        execute_shell(&format!(
            "iptables -D INPUT -s {} -j DROP 2>/dev/null; iptables -D FORWARD -s {} -j DROP 2>/dev/null",
            target, target
        ))?;
        self.persist()?;
        Ok(())
    }

    /// Persist rules to file
    fn persist(&self) -> Result<()> {
        execute_shell(&format!("iptables-save > {}", IPTABLES_RULES))
            .context("Failed to persist iptables rules")?;
        Ok(())
    }
}
