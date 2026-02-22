//! VPN (WireGuard) management commands

use anyhow::Result;
use serde::Serialize;
use std::fmt::Display;

use crate::adapters::wireguard;
use crate::output::print_output;

#[derive(Debug, Serialize)]
pub struct VpnPeersResult {
    pub interface_up: bool,
    pub listening_port: Option<u16>,
    pub peers: Vec<wireguard::WireguardPeer>,
}

impl Display for VpnPeersResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.interface_up {
            return writeln!(f, "WireGuard interface is down.");
        }
        writeln!(f, "WireGuard Peers (port {})", self.listening_port.unwrap_or(0))?;
        writeln!(f, "=========================")?;
        if self.peers.is_empty() {
            writeln!(f, "No peers configured.")?;
        } else {
            for peer in &self.peers {
                write!(f, "{}", peer)?;
            }
        }
        Ok(())
    }
}

/// List WireGuard peers with status
pub fn peers(json: bool) -> Result<()> {
    let status = wireguard::get_status()?;

    let result = VpnPeersResult {
        interface_up: status.interface_up,
        listening_port: status.listening_port,
        peers: status.peers,
    };

    print_output(&result, json);
    Ok(())
}

/// Show WireGuard status summary
pub fn status(json: bool) -> Result<()> {
    let status = wireguard::get_status()?;
    print_output(&status, json);
    Ok(())
}
