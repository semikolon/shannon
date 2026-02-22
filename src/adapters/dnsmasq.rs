//! dnsmasq configuration adapter

use anyhow::{Context, Result};
use serde::Serialize;
use std::net::IpAddr;

use crate::location::{execute_shell, read_file, systemctl, write_file};

const DNSMASQ_CUSTOM: &str = "/etc/dnsmasq.d/custom.conf";
const DNSMASQ_LEASES: &str = "/var/lib/misc/dnsmasq.leases";
const DNSMASQ_CONF: &str = "/etc/dnsmasq.conf";

#[derive(Debug, Serialize, Clone)]
pub struct DnsRecord {
    pub hostname: String,
    pub ip: IpAddr,
    pub source: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DhcpLease {
    pub mac: String,
    pub ip: String,
    pub hostname: String,
    pub expires: String,
    pub is_static: bool,
}

pub struct DnsmasqAdapter;

impl DnsmasqAdapter {
    pub fn new() -> Self {
        Self
    }

    /// List all DNS entries (address= lines)
    pub fn list_dns_entries(&self) -> Result<Vec<DnsRecord>> {
        let mut records = Vec::new();

        // Read main config
        if let Ok(content) = read_file(DNSMASQ_CONF) {
            records.extend(self.parse_dns_entries(&content, "main"));
        }

        // Read custom config
        if let Ok(content) = read_file(DNSMASQ_CUSTOM) {
            records.extend(self.parse_dns_entries(&content, "custom"));
        }

        // Read /etc/hosts
        if let Ok(content) = read_file("/etc/hosts") {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(ip) = parts[0].parse::<IpAddr>() {
                        for hostname in &parts[1..] {
                            if *hostname != "localhost" {
                                records.push(DnsRecord {
                                    hostname: hostname.to_string(),
                                    ip,
                                    source: "hosts".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(records)
    }

    fn parse_dns_entries(&self, content: &str, source: &str) -> Vec<DnsRecord> {
        let mut records = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("address=/") {
                // Format: address=/hostname/ip
                let parts: Vec<&str> = line.trim_start_matches("address=/").split('/').collect();
                if parts.len() >= 2 {
                    if let Ok(ip) = parts[1].parse::<IpAddr>() {
                        records.push(DnsRecord {
                            hostname: parts[0].to_string(),
                            ip,
                            source: source.to_string(),
                        });
                    }
                }
            }
        }

        records
    }

    /// Add a DNS entry to custom config
    pub fn add_dns_entry(&self, hostname: &str, ip: IpAddr) -> Result<()> {
        // Read existing custom config or create empty
        let existing = read_file(DNSMASQ_CUSTOM).unwrap_or_default();

        // Check for duplicates
        if existing.contains(&format!("address=/{}/", hostname)) {
            anyhow::bail!("DNS entry for {} already exists", hostname);
        }

        // Append new entry
        let new_content = format!("{}address=/{}/{}\n", existing, hostname, ip);
        write_file(DNSMASQ_CUSTOM, &new_content)?;

        // Reload dnsmasq
        self.reload()?;

        Ok(())
    }

    /// Remove a DNS entry from custom config
    pub fn remove_dns_entry(&self, hostname: &str) -> Result<()> {
        let content = read_file(DNSMASQ_CUSTOM)?;

        let pattern = format!("address=/{}/", hostname);
        let new_content: String = content
            .lines()
            .filter(|line| !line.contains(&pattern))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";

        if new_content == content {
            anyhow::bail!("DNS entry for {} not found in custom config", hostname);
        }

        write_file(DNSMASQ_CUSTOM, &new_content)?;
        self.reload()?;

        Ok(())
    }

    /// List DHCP leases
    pub fn list_leases(&self) -> Result<Vec<DhcpLease>> {
        let mut leases = Vec::new();

        // Parse active leases file
        // Format: expiry mac ip hostname client-id
        if let Ok(content) = read_file(DNSMASQ_LEASES) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    leases.push(DhcpLease {
                        expires: parts[0].to_string(),
                        mac: parts[1].to_string(),
                        ip: parts[2].to_string(),
                        hostname: parts[3].to_string(),
                        is_static: false,
                    });
                }
            }
        }

        // Parse static reservations from config (dhcp-host= lines)
        if let Ok(content) = read_file(DNSMASQ_CONF) {
            leases.extend(self.parse_static_reservations(&content));
        }

        Ok(leases)
    }

    fn parse_static_reservations(&self, content: &str) -> Vec<DhcpLease> {
        let mut reservations = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("dhcp-host=") {
                // Format: dhcp-host=mac,ip[,hostname]
                let value = line.trim_start_matches("dhcp-host=");
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() >= 2 {
                    reservations.push(DhcpLease {
                        mac: parts[0].to_string(),
                        ip: parts[1].to_string(),
                        hostname: parts.get(2).unwrap_or(&"").to_string(),
                        expires: "static".to_string(),
                        is_static: true,
                    });
                }
            }
        }

        reservations
    }

    /// Add a static DHCP reservation
    pub fn add_reservation(&self, mac: &str, ip: IpAddr, hostname: Option<&str>) -> Result<()> {
        let content = read_file(DNSMASQ_CONF)?;

        // Check for existing reservation
        if content.contains(mac) {
            anyhow::bail!("Reservation for MAC {} already exists", mac);
        }

        // Build new entry
        let entry = match hostname {
            Some(h) => format!("dhcp-host={},{},{}", mac, ip, h),
            None => format!("dhcp-host={},{}", mac, ip),
        };

        // Append to config
        let new_content = format!("{}\n{}\n", content.trim_end(), entry);
        write_file(DNSMASQ_CONF, &new_content)?;

        self.reload()?;
        Ok(())
    }

    /// Remove a DHCP reservation
    pub fn remove_reservation(&self, target: &str) -> Result<()> {
        let content = read_file(DNSMASQ_CONF)?;

        let new_content: String = content
            .lines()
            .filter(|line| {
                if line.starts_with("dhcp-host=") {
                    !line.contains(target)
                } else {
                    true
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";

        if new_content == content {
            anyhow::bail!("Reservation for {} not found", target);
        }

        write_file(DNSMASQ_CONF, &new_content)?;
        self.reload()?;

        Ok(())
    }

    /// Reload dnsmasq service
    fn reload(&self) -> Result<()> {
        systemctl("reload", "dnsmasq").context("Failed to reload dnsmasq")
    }
}
