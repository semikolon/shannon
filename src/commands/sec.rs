//! Security analysis and status commands

use anyhow::Result;
use serde::Serialize;
use std::fmt::Display;

use crate::adapters::{adguard, crowdsec, wireguard};
use crate::output::print_output;

/// Combined security stack status
#[derive(Debug, Serialize)]
pub struct SecurityStatus {
    pub adguard: adguard::AdguardStatus,
    pub crowdsec: crowdsec::CrowdsecStatus,
    pub wireguard: wireguard::WireguardStatus,
}

impl Display for SecurityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Security Stack Status")?;
        writeln!(f, "=====================")?;
        write!(f, "{}", self.adguard)?;
        write!(f, "{}", self.crowdsec)?;
        write!(f, "{}", self.wireguard)?;
        Ok(())
    }
}

/// Show combined security status
pub fn status(json: bool) -> Result<()> {
    let result = SecurityStatus {
        adguard: adguard::get_status()?,
        crowdsec: crowdsec::get_status()?,
        wireguard: wireguard::get_status()?,
    };

    print_output(&result, json);
    Ok(())
}

/// Show CrowdSec active decisions (blocked IPs)
pub fn blocks(json: bool) -> Result<()> {
    let decisions = crowdsec::list_decisions()?;

    let result = BlocksResult {
        count: decisions.len() as u32,
        decisions,
    };

    print_output(&result, json);
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct BlocksResult {
    pub count: u32,
    pub decisions: Vec<crowdsec::CrowdsecDecision>,
}

impl Display for BlocksResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CrowdSec Active Blocks")?;
        writeln!(f, "======================")?;
        if self.decisions.is_empty() {
            writeln!(f, "No active blocks.")?;
        } else {
            writeln!(f, "{} blocked IPs:", self.count)?;
            writeln!(f)?;
            for decision in &self.decisions {
                write!(f, "{}", decision)?;
            }
        }
        Ok(())
    }
}

/// Security finding from log analysis
#[derive(Debug, Serialize, Clone)]
pub struct SecurityFinding {
    pub timestamp: String,
    pub severity: String,
    pub category: String,
    pub summary: String,
    pub details: String,
}

impl Display for SecurityFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indicator = match self.severity.as_str() {
            "critical" | "high" => "🔴",
            "medium" => "🟡",
            _ => "🟢",
        };
        writeln!(f, "{} [{}] {}", indicator, self.severity.to_uppercase(), self.summary)?;
        writeln!(f, "   Category: {}", self.category)?;
        writeln!(f, "   Time: {}", self.timestamp)?;
        if !self.details.is_empty() {
            writeln!(f, "   Details: {}", self.details)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub findings: Vec<SecurityFinding>,
    pub logs_analyzed: usize,
    pub time_window_hours: u32,
}

impl Display for ScanResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Security Scan Results")?;
        writeln!(f, "=====================")?;
        writeln!(f, "Analyzed {} log entries (last {} hours)", self.logs_analyzed, self.time_window_hours)?;
        writeln!(f)?;

        if self.findings.is_empty() {
            writeln!(f, "No security issues detected.")?;
        } else {
            writeln!(f, "Found {} issues:", self.findings.len())?;
            writeln!(f)?;
            for finding in &self.findings {
                write!(f, "{}", finding)?;
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

/// Run security scan (LLM-based analysis — placeholder)
pub fn scan(json: bool) -> Result<()> {
    let result = ScanResult {
        findings: vec![],
        logs_analyzed: 0,
        time_window_hours: 1,
    };

    print_output(&result, json);
    println!("\nNote: LLM-based security analysis not yet implemented.");
    println!("Run 'shannon sec blocks' to view active CrowdSec decisions.");

    Ok(())
}

/// Show recent security findings
pub fn report(hours: u32, json: bool) -> Result<()> {
    let result = ScanResult {
        findings: vec![],
        logs_analyzed: 0,
        time_window_hours: hours,
    };

    print_output(&result, json);
    println!("\nNote: No findings stored yet. Run 'shannon sec scan' first.");

    Ok(())
}
