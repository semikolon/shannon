//! CrowdSec IDS adapter — wraps `cscli` commands

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::location::execute_shell;

#[derive(Debug, Serialize)]
pub struct CrowdsecStatus {
    pub running: bool,
    pub active_decisions: u32,
    pub scenarios_loaded: u32,
}

impl Display for CrowdsecStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.running {
            return writeln!(f, "  CrowdSec: not running");
        }
        writeln!(f, "  CrowdSec: active ({} decisions, {} scenarios)",
            self.active_decisions, self.scenarios_loaded)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrowdsecDecision {
    pub id: u64,
    #[serde(alias = "value")]
    pub source_ip: String,
    #[serde(alias = "scenario")]
    pub reason: String,
    #[serde(alias = "type")]
    pub action: String,
    pub duration: String,
    pub origin: String,
}

impl Display for CrowdsecDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  {} — {} ({})", self.source_ip, self.reason, self.duration)?;
        writeln!(f, "     Action: {} | Origin: {}", self.action, self.origin)?;
        Ok(())
    }
}

/// Check if CrowdSec engine is running
pub fn get_status() -> Result<CrowdsecStatus> {
    let running = execute_shell("systemctl is-active crowdsec 2>/dev/null")
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !running {
        return Ok(CrowdsecStatus {
            running: false,
            active_decisions: 0,
            scenarios_loaded: 0,
        });
    }

    let decisions = count_decisions().unwrap_or(0);
    let scenarios = count_scenarios().unwrap_or(0);

    Ok(CrowdsecStatus {
        running,
        active_decisions: decisions,
        scenarios_loaded: scenarios,
    })
}

/// List active CrowdSec decisions
pub fn list_decisions() -> Result<Vec<CrowdsecDecision>> {
    let output = execute_shell("cscli decisions list -o json 2>/dev/null")?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();

    if text.is_empty() || text == "null" {
        return Ok(vec![]);
    }

    let decisions: Vec<CrowdsecDecision> = serde_json::from_str(text)
        .unwrap_or_default();

    Ok(decisions)
}

fn count_decisions() -> Result<u32> {
    let output = execute_shell("cscli decisions list -o json 2>/dev/null")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();

    if text.is_empty() || text == "null" {
        return Ok(0);
    }

    let arr: Vec<serde_json::Value> = serde_json::from_str(text).unwrap_or_default();
    Ok(arr.len() as u32)
}

fn count_scenarios() -> Result<u32> {
    let output = execute_shell("cscli scenarios list -o json 2>/dev/null")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();

    if text.is_empty() || text == "null" {
        return Ok(0);
    }

    let arr: Vec<serde_json::Value> = serde_json::from_str(text).unwrap_or_default();
    Ok(arr.len() as u32)
}
