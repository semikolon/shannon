//! DHCP lease and reservation management

use anyhow::Result;
use serde::Serialize;
use std::fmt::Display;
use std::net::IpAddr;

use crate::adapters::dnsmasq::{DhcpLease, DnsmasqAdapter};
use crate::output::{confirm, print_output, TableOutput, TableRow};

impl TableRow for DhcpLease {
    fn cells(&self) -> Vec<String> {
        vec![
            self.mac.clone(),
            self.ip.clone(),
            self.hostname.clone(),
            self.expires.clone(),
        ]
    }
}


/// List DHCP leases
pub fn leases(json: bool) -> Result<()> {
    let adapter = DnsmasqAdapter::new();
    let leases = adapter.list_leases()?;

    let output = TableOutput {
        headers: vec![
            "MAC".to_string(),
            "IP".to_string(),
            "Hostname".to_string(),
            "Expires".to_string(),
        ],
        rows: leases,
    };

    print_output(&output, json);
    Ok(())
}

/// Add a DHCP reservation
pub fn reserve(mac: &str, ip: IpAddr, hostname: Option<&str>, yes: bool, json: bool) -> Result<()> {
    let adapter = DnsmasqAdapter::new();

    if !yes && !confirm(&format!("Add reservation for {} -> {}?", mac, ip), yes) {
        anyhow::bail!("Operation cancelled");
    }

    adapter.add_reservation(mac, ip, hostname)?;

    #[derive(Serialize)]
    struct ReserveResult {
        success: bool,
        mac: String,
        ip: String,
    }

    impl Display for ReserveResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Added reservation: {} -> {}", self.mac, self.ip)
        }
    }

    let result = ReserveResult {
        success: true,
        mac: mac.to_string(),
        ip: ip.to_string(),
    };

    print_output(&result, json);
    Ok(())
}

/// Remove a DHCP reservation
pub fn unreserve(target: &str, yes: bool, json: bool) -> Result<()> {
    let adapter = DnsmasqAdapter::new();

    if !yes && !confirm(&format!("Remove reservation for {}?", target), yes) {
        anyhow::bail!("Operation cancelled");
    }

    adapter.remove_reservation(target)?;

    #[derive(Serialize)]
    struct UnreserveResult {
        success: bool,
        target: String,
    }

    impl Display for UnreserveResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Removed reservation for {}", self.target)
        }
    }

    let result = UnreserveResult {
        success: true,
        target: target.to_string(),
    };

    print_output(&result, json);
    Ok(())
}
