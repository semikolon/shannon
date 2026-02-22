//! AdGuard Home adapter — wraps REST API on localhost:3000

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::location::execute_shell;

#[derive(Debug, Serialize)]
pub struct AdguardStatus {
    pub running: bool,
    pub dns_queries_today: u64,
    pub blocked_today: u64,
    pub blocklist_count: u32,
}

impl Display for AdguardStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.running {
            return writeln!(f, "  AdGuard Home: not running");
        }
        writeln!(f, "  AdGuard Home: active ({} queries, {} blocked, {} rules)",
            self.dns_queries_today, self.blocked_today, self.blocklist_count)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct AdguardStatsResponse {
    num_dns_queries: Option<u64>,
    num_blocked_filtering: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AdguardFilterStatus {
    filters: Option<Vec<AdguardFilter>>,
}

#[derive(Debug, Deserialize)]
struct AdguardFilter {
    rules_count: Option<u32>,
    enabled: Option<bool>,
}

/// Get AdGuard Home status via REST API
pub fn get_status() -> Result<AdguardStatus> {
    let running = execute_shell("systemctl is-active AdGuardHome 2>/dev/null")
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !running {
        return Ok(AdguardStatus {
            running: false,
            dns_queries_today: 0,
            blocked_today: 0,
            blocklist_count: 0,
        });
    }

    let (queries, blocked) = get_stats().unwrap_or((0, 0));
    let rules = get_filter_rules().unwrap_or(0);

    Ok(AdguardStatus {
        running,
        dns_queries_today: queries,
        blocked_today: blocked,
        blocklist_count: rules,
    })
}

fn get_stats() -> Result<(u64, u64)> {
    let output = execute_shell(
        "curl -s -u admin:shannon-admin-2026 http://localhost:3000/control/stats"
    )?;

    if !output.status.success() {
        return Ok((0, 0));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let stats: AdguardStatsResponse = serde_json::from_str(&text).unwrap_or(AdguardStatsResponse {
        num_dns_queries: None,
        num_blocked_filtering: None,
    });

    Ok((
        stats.num_dns_queries.unwrap_or(0),
        stats.num_blocked_filtering.unwrap_or(0),
    ))
}

fn get_filter_rules() -> Result<u32> {
    let output = execute_shell(
        "curl -s -u admin:shannon-admin-2026 http://localhost:3000/control/filtering/status"
    )?;

    if !output.status.success() {
        return Ok(0);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let status: AdguardFilterStatus = serde_json::from_str(&text).unwrap_or(AdguardFilterStatus {
        filters: None,
    });

    let count = status.filters
        .unwrap_or_default()
        .iter()
        .filter(|f| f.enabled.unwrap_or(false))
        .map(|f| f.rules_count.unwrap_or(0))
        .sum();

    Ok(count)
}
