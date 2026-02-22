//! Firewall and port forwarding management

use anyhow::Result;
use serde::Serialize;
use std::fmt::Display;

use crate::adapters::nftables::{NftablesAdapter, PortForward};
use crate::output::{confirm, print_output, TableOutput, TableRow};

impl TableRow for PortForward {
    fn cells(&self) -> Vec<String> {
        vec![
            self.external_port.to_string(),
            format!("{}:{}", self.internal_ip, self.internal_port),
            self.protocol.clone(),
            self.comment.clone().unwrap_or_default(),
        ]
    }
}


/// List firewall rules
pub fn list(json: bool) -> Result<()> {
    let adapter = NftablesAdapter::new();
    let forwards = adapter.list_port_forwards()?;

    let output = TableOutput {
        headers: vec![
            "External".to_string(),
            "Internal".to_string(),
            "Proto".to_string(),
            "Comment".to_string(),
        ],
        rows: forwards,
    };

    print_output(&output, json);
    Ok(())
}

/// Add a port forward
pub fn forward(
    external_port: u16,
    internal: &str,
    proto: &str,
    yes: bool,
    json: bool,
) -> Result<()> {
    let adapter = NftablesAdapter::new();

    // Parse internal target (ip:port)
    let parts: Vec<&str> = internal.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Internal target must be in ip:port format");
    }

    let internal_ip = parts[0].to_string();
    let internal_port: u16 = parts[1].parse()?;

    if !yes
        && !confirm(
            &format!(
                "Forward port {} -> {}:{} ({})?",
                external_port, internal_ip, internal_port, proto
            ),
            yes,
        )
    {
        anyhow::bail!("Operation cancelled");
    }

    let rule = PortForward {
        external_port,
        internal_ip,
        internal_port,
        protocol: proto.to_string(),
        comment: None,
    };

    adapter.add_port_forward(&rule)?;

    #[derive(Serialize)]
    struct ForwardResult {
        success: bool,
        external_port: u16,
        internal: String,
        protocol: String,
    }

    impl Display for ForwardResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Added port forward: {} -> {} ({})",
                self.external_port, self.internal, self.protocol
            )
        }
    }

    let result = ForwardResult {
        success: true,
        external_port,
        internal: internal.to_string(),
        protocol: proto.to_string(),
    };

    print_output(&result, json);
    Ok(())
}

/// Remove a port forward
pub fn unforward(external_port: u16, yes: bool, json: bool) -> Result<()> {
    let adapter = NftablesAdapter::new();

    if !yes && !confirm(&format!("Remove forward for port {}?", external_port), yes) {
        anyhow::bail!("Operation cancelled");
    }

    adapter.remove_port_forward(external_port)?;

    #[derive(Serialize)]
    struct UnforwardResult {
        success: bool,
        external_port: u16,
    }

    impl Display for UnforwardResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Removed forward for port {}", self.external_port)
        }
    }

    let result = UnforwardResult {
        success: true,
        external_port,
    };

    print_output(&result, json);
    Ok(())
}

/// Block an IP
pub fn block(target: &str, yes: bool, json: bool) -> Result<()> {
    let adapter = NftablesAdapter::new();

    if !yes && !confirm(&format!("Block {}?", target), yes) {
        anyhow::bail!("Operation cancelled");
    }

    adapter.block_ip(target)?;

    #[derive(Serialize)]
    struct BlockResult {
        success: bool,
        target: String,
    }

    impl Display for BlockResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Blocked {}", self.target)
        }
    }

    let result = BlockResult {
        success: true,
        target: target.to_string(),
    };

    print_output(&result, json);
    Ok(())
}

/// Unblock an IP
pub fn unblock(target: &str, yes: bool, json: bool) -> Result<()> {
    let adapter = NftablesAdapter::new();

    if !yes && !confirm(&format!("Unblock {}?", target), yes) {
        anyhow::bail!("Operation cancelled");
    }

    adapter.unblock_ip(target)?;

    #[derive(Serialize)]
    struct UnblockResult {
        success: bool,
        target: String,
    }

    impl Display for UnblockResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Unblocked {}", self.target)
        }
    }

    let result = UnblockResult {
        success: true,
        target: target.to_string(),
    };

    print_output(&result, json);
    Ok(())
}
