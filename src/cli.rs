//! CLI argument definitions using Clap derive macros

use clap::{Parser, Subcommand};
use std::net::IpAddr;

#[derive(Parser)]
#[command(
    name = "shannon",
    about = "Unified router management CLI for Rock Pi 4B SE",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output in JSON format (for AI agents)
    #[arg(long, global = true)]
    pub json: bool,

    /// Skip confirmation prompts
    #[arg(long, short = 'y', global = true)]
    pub yes: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// System health and diagnostics
    Status {
        #[command(subcommand)]
        action: Option<StatusAction>,
    },

    /// DNS record management
    Dns {
        #[command(subcommand)]
        action: DnsAction,
    },

    /// DHCP lease and reservation management
    Dhcp {
        #[command(subcommand)]
        action: DhcpAction,
    },

    /// Firewall and port forwarding
    Fw {
        #[command(subcommand)]
        action: FwAction,
    },

    /// Security analysis
    Sec {
        #[command(subcommand)]
        action: SecAction,
    },
}

// Status subcommands
#[derive(Subcommand)]
pub enum StatusAction {
    /// Run diagnostic checks (DNS, gateway, internet)
    Doctor,
}

// DNS subcommands
#[derive(Subcommand)]
pub enum DnsAction {
    /// List all DNS records
    List,

    /// Add a DNS record
    Add {
        /// Hostname to add
        hostname: String,
        /// IP address to point to
        ip: IpAddr,
    },

    /// Remove a DNS record
    Rm {
        /// Hostname to remove
        hostname: String,
    },
}

// DHCP subcommands
#[derive(Subcommand)]
pub enum DhcpAction {
    /// List all DHCP leases
    Leases,

    /// Add a static DHCP reservation
    Reserve {
        /// MAC address (format: aa:bb:cc:dd:ee:ff)
        mac: String,
        /// IP address to reserve
        ip: IpAddr,
        /// Optional hostname
        #[arg(short, long)]
        hostname: Option<String>,
    },

    /// Remove a DHCP reservation
    Unreserve {
        /// MAC address or IP to unreserve
        target: String,
    },
}

// Firewall subcommands
#[derive(Subcommand)]
pub enum FwAction {
    /// List firewall rules and port forwards
    List,

    /// Add a port forwarding rule
    Forward {
        /// External port to forward
        external_port: u16,
        /// Internal destination (ip:port format)
        internal: String,
        /// Protocol (tcp, udp, or both)
        #[arg(short, long, default_value = "tcp")]
        proto: String,
    },

    /// Remove a port forwarding rule
    Unforward {
        /// External port to stop forwarding
        external_port: u16,
    },

    /// Block an IP address or range
    Block {
        /// IP address or CIDR range to block
        target: String,
    },

    /// Unblock an IP address or range
    Unblock {
        /// IP address or CIDR range to unblock
        target: String,
    },
}

// Security subcommands
#[derive(Subcommand)]
pub enum SecAction {
    /// Run security analysis on logs
    Scan,

    /// Show recent security findings
    Report {
        /// Number of hours to look back (default: 24)
        #[arg(short = 'n', long, default_value = "24")]
        hours: u32,
    },
}
