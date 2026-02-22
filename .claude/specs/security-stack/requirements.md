# Requirements: SHANNON Security Stack (Phase 2)

## Overview

- **Type**: Integration
- **Problem**: Install and configure AdGuard Home (DNS filtering), CrowdSec (IDS with nftables bouncer), and WireGuard (VPN) on Rock Pi 4B SE running Armbian Trixie
- **Pain Point**: No visibility into attacks/ads/trackers, manual log checking, no remote management capability
- **Success Unlocks**: Autonomous security operations, enables future expansion (NAS, smart home), peace of mind with remote access

---

## User Stories

### US-1: DNS-Level Filtering (AdGuard Home)

**As a** network administrator managing SHANNON
**I want** all DNS queries filtered through AdGuard Home
**So that** ads, trackers, and known-malicious domains are blocked network-wide before connections are made

**Acceptance Criteria:**
- [ ] AC-1.1: dnsmasq DNS disabled (`port=0`), DHCP only on ports 67/68
- [ ] AC-1.2: AdGuard Home running on port 53, all DHCP clients use it for DNS
- [ ] AC-1.3: DoH/DoT upstream configured (Cloudflare or Quad9)
- [ ] AC-1.4: Blocklists loaded (OISD, AdGuard default, malware domains)
- [ ] AC-1.5: Visible result: YouTube/web ads blocked on all network devices
- [ ] AC-1.6: No duplicated DNS functionality (dnsmasq DNS completely off)

**EARS Constraints:**
- **When** AdGuard Home fails, **the system shall** allow clients to reach upstream DNS directly (fail-safe)
- **While** normal operation, **the system shall** log blocked queries for later review
- **If** a legitimate domain is blocked, **then the system shall** allow whitelisting via CLI or web UI

### US-2: Proactive Threat Defense (CrowdSec)

**As a** network administrator
**I want** known-bad IPs blocked before they attack my network
**So that** I benefit from community threat intelligence without manual log analysis

**Acceptance Criteria:**
- [ ] AC-2.1: CrowdSec engine installed and monitoring SSH logs (`/var/log/auth.log`)
- [ ] AC-2.2: nftables bouncer blocking IPs in CrowdSec's blocklist
- [ ] AC-2.3: Community blocklist enabled and syncing automatically
- [ ] AC-2.4: `cscli metrics` and `cscli decisions list` functional
- [ ] AC-2.5: Significant alerts route through ntfy-bridge (existing) → TTS daemon → Ruby narrates
- [ ] AC-2.6: Escalation to Claude Code possible via copyable prompt

**EARS Constraints:**
- **When** CrowdSec engine fails, **the system shall** allow all traffic (fail-open) and notify admin via ntfy
- **While** normal operation, **the system shall** sync community blocklist automatically
- **If** a legitimate IP is blocked (false positive), **then the system shall** allow whitelisting via `cscli decisions delete`

### US-3: Remote Access (WireGuard)

**As a** solo administrator who travels
**I want** encrypted VPN access to my home network from anywhere
**So that** I can manage SHANNON and access home services when away

**Acceptance Criteria:**
- [ ] AC-3.1: WireGuard kernel module loaded, `wg0` interface configured
- [ ] AC-3.2: Server config at `/etc/wireguard/wg0.conf` with static port (51820)
- [x] AC-3.3: ~~Port forwarding~~ Not needed — SHANNON has public IP directly on WAN interface
- [ ] AC-3.4: Peer configs generated for: iPhone, MacBook Pro, Mac Mini, future placeholder
- [ ] AC-3.5: Can connect from mobile data and complete `ssh shannon` successfully
- [ ] AC-3.6: DNS through tunnel routes to AdGuard Home (192.168.4.1)

**EARS Constraints:**
- **When** VPN connection established, **the system shall** route all peer traffic through SHANNON
- **If** WireGuard port unreachable, **then the system shall** allow graceful timeout (no crash)

### US-4: CLI Integration (shannon sec)

**As a** CLI-first administrator
**I want** basic security status and operations via `shannon` CLI
**So that** I can check health and perform common ops without learning three separate tools

**Acceptance Criteria:**
- [ ] AC-4.1: `shannon sec status` shows health of AdGuard, CrowdSec, WireGuard
- [ ] AC-4.2: `shannon sec blocks` shows recent CrowdSec decisions
- [ ] AC-4.3: `shannon vpn peers` lists WireGuard peers with handshake status
- [ ] AC-4.4: `shannon vpn add <name>` generates peer config and QR code
- [ ] AC-4.5: JSON output (`--json`) works for all commands (AI agent friendly)
- [ ] AC-4.6: Native tools (`cscli`, `wg`, AdGuard web UI) remain accessible for advanced ops

**EARS Constraints:**
- **When** underlying service unavailable, **the system shall** report clear error (not crash)

### US-5: Configuration Management

**As an** administrator who values reproducibility
**I want** security stack configs tracked in version control
**So that** I can restore or audit changes over time

**Acceptance Criteria:**
- [ ] AC-5.1: `/etc/wireguard/` configs synced to private git repo
- [ ] AC-5.2: `/etc/crowdsec/config.yaml` tracked
- [ ] AC-5.3: AdGuard `AdGuardHome.yaml` tracked
- [ ] AC-5.4: Sensitive keys stored securely (not in plain git, or private repo only)

### US-6: LLM Security Analysis (Dual-Layer)

**As a** security-conscious administrator
**I want** LLMs to analyze raw system logs for patterns CrowdSec misses — quick hourly triage plus deep daily analysis
**So that** novel attack patterns, multi-step correlation, and behavioral anomalies are caught by semantic reasoning, not just signature matching

**Acceptance Criteria:**
- [ ] AC-6.1: Hourly cron sends last hour's logs to GPT-5-nano for quick triage (~$0.72/month)
- [ ] AC-6.2: Daily cron sends last 24h logs to Gemini 3 Pro for deep analysis (~$2.10/month)
- [ ] AC-6.3: Critical hourly findings → ntfy urgent → TTS/Ruby narrates immediately
- [ ] AC-6.4: Normal hourly findings → appended to daily digest file
- [ ] AC-6.5: Daily analysis produces pattern correlations, behavioral anomalies, trend detection
- [ ] AC-6.6: Escalation prompt generated for Claude Code investigation of significant findings
- [ ] AC-6.7: Total LLM cost stays under $5/month (~$2.82/month expected)
- [ ] AC-6.8: LLM analysis complements CrowdSec — neither replaces the other

**EARS Constraints:**
- **When** LLM API unavailable, **the system shall** log the failure and retry next cycle (CrowdSec continues independently)
- **While** normal operation, **the system shall** log each hourly triage run (confirms system is running)

**Rationale:** Inspired by Anthropic's Claude Code Security (agentic Observe→Hypothesize→Test→Refine→Verify loops finding 500+ zero-days). CrowdSec handles known patterns; LLM reasoning catches what signatures miss.

---

## Non-Functional Requirements

### Performance
- Combined stack RAM: <400 MB idle (leaving >3.5 GB headroom on 4 GB device)
- DNS resolution latency: <50ms for cached queries
- WireGuard throughput: Near line-rate on gigabit (kernel-space crypto)

### Error Handling
- **Philosophy**: Fail-safe AND notify — maintain network connectivity while alerting admin
- **Anticipated failures**: DNS misconfiguration, overzealous CrowdSec blocking, WireGuard port forwarding issues
- **Recovery**: IP-based SSH (192.168.4.1) always works regardless of DNS state

### Success Criteria
- **Gut check**: Ads visibly blocked + CrowdSec showing blocked IPs + VPN works from mobile data
- **Embarrassment criteria**: Network breaking, false positives blocking legitimate traffic, resource exhaustion, duplicated functionality

---

## Out of Scope (Phase 2)

- Suricata/deep packet inspection (deferred to Dell if ever)
- Grafana/Prometheus on SHANNON (Dell has Prometheus)
- Full CLI wrapper for all AdGuard/CrowdSec features (use native tools for advanced ops)
- High availability / failover (single point of failure accepted)
- Automated WireGuard key rotation (manual rotation is fine for 4 peers)

---

## Risks & Assumptions

- **Assumption: ARM64 works** — AdGuard and CrowdSec have official ARM64 packages; WireGuard is in-kernel
- **Assumption: 4 GB is enough** — Research confirms ~400 MB stack overhead
- **Assumption: "set and forget"** — Minimal admin overhead expected (~5 min/week for CrowdSec hub updates)
- **Risk: dnsmasq transition** — Mitigated by incremental testing, IP-based SSH fallback
- **Conscious debt**: Manual WG key management, single point of failure, some configs not gitified initially

---

## Open Questions

1. ~~**ISP router port forwarding**~~: RESOLVED — Not needed. SHANNON has public IP (94.254.88.116) directly on WAN interface. WireGuard is directly reachable.
2. ~~**CrowdSec notification integration**~~: RESOLVED — ntfy-bridge (`~/Projects/ntfy-bridge`) already exists as the gateway. Flow: SHANNON → ntfy server (Dell:8099) → ntfy-bridge (Mac) → TTS daemon socket → Ruby narrates. **Blocked**: Dell unreachable at 192.168.4.84 (ARP FAILED, likely old Deco-era network config).
3. **Config git sync**: Manual rsync or automated cron job?
