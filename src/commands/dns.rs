//! DNS record management

use anyhow::Result;
use serde::Serialize;
use std::fmt::Display;
use std::net::IpAddr;

use crate::adapters::dnsmasq::{DnsmasqAdapter, DnsRecord};
use crate::output::{print_output, TableOutput, TableRow};

impl TableRow for DnsRecord {
    fn cells(&self) -> Vec<String> {
        vec![
            self.hostname.clone(),
            self.ip.to_string(),
            self.source.clone(),
        ]
    }
}


/// List DNS records
pub fn list(json: bool) -> Result<()> {
    let adapter = DnsmasqAdapter::new();
    let records = adapter.list_dns_entries()?;

    let output = TableOutput {
        headers: vec![
            "Hostname".to_string(),
            "IP".to_string(),
            "Source".to_string(),
        ],
        rows: records,
    };

    print_output(&output, json);
    Ok(())
}

/// Add a DNS record
pub fn add(hostname: &str, ip: IpAddr, json: bool) -> Result<()> {
    let adapter = DnsmasqAdapter::new();
    adapter.add_dns_entry(hostname, ip)?;

    #[derive(Serialize)]
    struct AddResult {
        success: bool,
        hostname: String,
        ip: String,
    }

    impl Display for AddResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Added DNS record: {} -> {}", self.hostname, self.ip)
        }
    }

    let result = AddResult {
        success: true,
        hostname: hostname.to_string(),
        ip: ip.to_string(),
    };

    print_output(&result, json);
    Ok(())
}

/// Remove a DNS record
pub fn remove(hostname: &str, json: bool) -> Result<()> {
    let adapter = DnsmasqAdapter::new();
    adapter.remove_dns_entry(hostname)?;

    #[derive(Serialize)]
    struct RemoveResult {
        success: bool,
        hostname: String,
    }

    impl Display for RemoveResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Removed DNS record: {}", self.hostname)
        }
    }

    let result = RemoveResult {
        success: true,
        hostname: hostname.to_string(),
    };

    print_output(&result, json);
    Ok(())
}
