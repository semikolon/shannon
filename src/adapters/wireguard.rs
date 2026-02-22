//! WireGuard VPN adapter — wraps `wg show` output

use anyhow::Result;
use serde::Serialize;
use std::fmt::Display;

use crate::location::execute_shell;

#[derive(Debug, Serialize)]
pub struct WireguardStatus {
    pub interface_up: bool,
    pub listening_port: Option<u16>,
    pub public_key: String,
    pub peers: Vec<WireguardPeer>,
}

#[derive(Debug, Serialize)]
pub struct WireguardPeer {
    pub name: String,
    pub public_key: String,
    pub allowed_ips: String,
    pub last_handshake: Option<String>,
    pub transfer_rx: String,
    pub transfer_tx: String,
    pub connected: bool,
}

impl Display for WireguardStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.interface_up {
            return writeln!(f, "  WireGuard: down");
        }
        writeln!(f, "  WireGuard: up (port {})", self.listening_port.unwrap_or(0))?;
        writeln!(f, "  Peers: {}/{} connected",
            self.peers.iter().filter(|p| p.connected).count(),
            self.peers.len()
        )?;
        Ok(())
    }
}

impl Display for WireguardPeer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indicator = if self.connected { "●" } else { "○" };
        writeln!(f, "{} {} ({})", indicator, self.name, self.allowed_ips)?;
        if let Some(ref hs) = self.last_handshake {
            writeln!(f, "    Last handshake: {}", hs)?;
        }
        writeln!(f, "    Transfer: {} rx / {} tx", self.transfer_rx, self.transfer_tx)?;
        Ok(())
    }
}

/// Get WireGuard status by parsing `wg show wg0`
pub fn get_status() -> Result<WireguardStatus> {
    let output = execute_shell("wg show wg0 2>/dev/null")?;

    if !output.status.success() {
        return Ok(WireguardStatus {
            interface_up: false,
            listening_port: None,
            public_key: String::new(),
            peers: vec![],
        });
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut public_key = String::new();
    let mut listening_port = None;
    let mut peers = Vec::new();
    let mut current_peer: Option<PeerBuilder> = None;

    for line in text.lines() {
        let line = line.trim();

        if line.starts_with("public key:") {
            if current_peer.is_none() {
                public_key = line.trim_start_matches("public key:").trim().to_string();
            }
        } else if line.starts_with("listening port:") {
            listening_port = line.trim_start_matches("listening port:")
                .trim()
                .parse()
                .ok();
        } else if line.starts_with("peer:") {
            if let Some(builder) = current_peer.take() {
                peers.push(builder.build());
            }
            current_peer = Some(PeerBuilder {
                public_key: line.trim_start_matches("peer:").trim().to_string(),
                ..PeerBuilder::default()
            });
        } else if let Some(ref mut builder) = current_peer {
            if line.starts_with("allowed ips:") {
                builder.allowed_ips = line.trim_start_matches("allowed ips:").trim().to_string();
            } else if line.starts_with("latest handshake:") {
                builder.last_handshake = Some(
                    line.trim_start_matches("latest handshake:").trim().to_string()
                );
            } else if line.starts_with("transfer:") {
                let transfer = line.trim_start_matches("transfer:").trim();
                if let Some((rx, tx)) = transfer.split_once("received,") {
                    builder.transfer_rx = rx.trim().to_string();
                    builder.transfer_tx = tx.trim_end_matches("sent").trim().to_string();
                }
            }
        }
    }

    if let Some(builder) = current_peer.take() {
        peers.push(builder.build());
    }

    // Resolve peer names from wg0.conf comments
    let names = resolve_peer_names();
    for peer in &mut peers {
        if let Some(name) = names.get(&peer.public_key) {
            peer.name = name.clone();
        }
    }

    Ok(WireguardStatus {
        interface_up: true,
        listening_port,
        public_key,
        peers,
    })
}

#[derive(Default)]
struct PeerBuilder {
    public_key: String,
    allowed_ips: String,
    last_handshake: Option<String>,
    transfer_rx: String,
    transfer_tx: String,
}

impl PeerBuilder {
    fn build(self) -> WireguardPeer {
        let connected = self.last_handshake.is_some();
        WireguardPeer {
            name: short_key(&self.public_key),
            public_key: self.public_key,
            allowed_ips: self.allowed_ips,
            last_handshake: self.last_handshake,
            transfer_rx: if self.transfer_rx.is_empty() { "0 B".into() } else { self.transfer_rx },
            transfer_tx: if self.transfer_tx.is_empty() { "0 B".into() } else { self.transfer_tx },
            connected,
        }
    }
}

/// Resolve peer names from wg0.conf comment lines (e.g. "# iPhone")
fn resolve_peer_names() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    let Ok(output) = execute_shell("cat /etc/wireguard/wg0.conf 2>/dev/null") else {
        return map;
    };

    if !output.status.success() {
        return map;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut last_comment = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            last_comment = trimmed.trim_start_matches('#').trim().to_string();
        } else if trimmed.starts_with("PublicKey") {
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[pos + 1..].trim().to_string();
                if !last_comment.is_empty() {
                    map.insert(key, last_comment.clone());
                }
            }
            last_comment.clear();
        }
    }

    map
}

fn short_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...", &key[..8])
    } else {
        key.to_string()
    }
}
