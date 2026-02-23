# Design: SHANNON Security Stack (Phase 2)

## Tech Stack

- **Target Platform**: Rock Pi 4B SE, Armbian Trixie, kernel 6.18, ARM64, 4 GB RAM
- **Existing**: dnsmasq (DHCP+DNS), nftables, ssh
- **Adding**: AdGuard Home, CrowdSec + nftables bouncer, WireGuard (kernel module)
- **CLI Integration**: Extend existing `shannon` Rust CLI (Clap 4, anyhow, serde)

---

## Architecture Overview

```
                           ┌─────────────────────────────────────────────────────┐
                           │                     SHANNON                         │
                           │              Rock Pi 4B SE (Armbian)                │
Internet ──►               │                                                     │
              ┌────────────┼─────────────────────────────────────────────────────┼────────────┐
              │            │                                                     │            │
              │ WireGuard  │   ┌───────────────┐      ┌──────────────────────┐  │  LAN       │
              │ :51820     │   │ CrowdSec      │      │ AdGuard Home         │  │  Clients   │
              │ ─ ─ ─ ─ ─ ─│───┤ Engine        │      │ :53 (DNS)            │  │  ─ ─ ─ ─ ─│
              │ Tunnel In  │   │ ↓             │      │ ↓                    │  │            │
              │            │   │ nftables      │      │ DoH/DoT upstream     │  │  Phone     │
              │            │   │ bouncer       │      │ (Cloudflare/Quad9)   │  │  Laptop    │
              │            │   │ ↓             │      └──────────────────────┘  │  TV        │
              │            │   │ Block IPs     │                                │  ...       │
              │            │   └───────────────┘      ┌──────────────────────┐  │            │
              │            │                          │ dnsmasq              │  │            │
              │            │                          │ :67/68 (DHCP only)   │  │            │
              │            │                          │ port=0 (DNS off)     │  │            │
              │            │                          └──────────────────────┘  │            │
              └────────────┼─────────────────────────────────────────────────────┼────────────┘
                           │                                                     │
                           │          ┌─────────────────────────────┐           │
                           │          │ shannon CLI                  │           │
                           │          │ sec status / sec blocks      │           │
                           │          │ vpn peers / vpn add          │           │
                           │          └─────────────────────────────┘           │
                           └─────────────────────────────────────────────────────┘
```

### Component Interaction

1. **DNS Flow**: Client → AdGuard Home (:53) → blocklist check → DoH upstream → response
2. **DHCP Flow**: Client → dnsmasq (:67) → lease (pushes AdGuard as DNS server)
3. **Threat Detection**: SSH logs → CrowdSec engine → community blocklist + local scenarios → nftables bouncer → block IP
4. **VPN Flow**: Remote device → WireGuard (:51820) → encrypted tunnel → LAN access + AdGuard DNS

---

## File Structure

### New Adapters (shannon CLI)

```
src/adapters/
├── adguard.rs      # AdGuard Home status via API (localhost:3000)
├── crowdsec.rs     # CrowdSec status via cscli wrapper
├── wireguard.rs    # WireGuard status via wg show, peer management
```

### CLI Extensions

```
src/cli.rs          # Add VpnAction subcommand
src/commands/
├── sec.rs          # Extend with status (AdGuard + CrowdSec health)
├── vpn.rs          # New: peers, add, remove
```

### System Configs (on SHANNON)

```
/etc/
├── dnsmasq.conf                  # Add: port=0, dhcp-option=6,192.168.4.1
├── AdGuardHome/
│   └── AdGuardHome.yaml          # Main config
├── crowdsec/
│   ├── config.yaml               # Main config
│   ├── acquis.yaml               # Log acquisition (auth.log, syslog)
│   └── bouncers/
│       └── crowdsec-firewall-bouncer.yaml
├── wireguard/
│   ├── wg0.conf                  # Server config
│   ├── peers/                    # Peer configs (for git tracking)
│   │   ├── iphone.conf
│   │   ├── mbp.conf
│   │   ├── macmini.conf
│   │   └── future.conf
```

---

## Data Models

### Security Status (shannon sec status)

```rust
#[derive(Debug, Serialize)]
pub struct SecurityStatus {
    pub adguard: AdguardStatus,
    pub crowdsec: CrowdsecStatus,
    pub wireguard: WireguardStatus,
}

#[derive(Debug, Serialize)]
pub struct AdguardStatus {
    pub running: bool,
    pub dns_queries_today: u64,
    pub blocked_today: u64,
    pub blocklist_count: u32,
}

#[derive(Debug, Serialize)]
pub struct CrowdsecStatus {
    pub running: bool,
    pub active_decisions: u32,
    pub scenarios_loaded: u32,
    pub last_sync: String,  // community blocklist
}

#[derive(Debug, Serialize)]
pub struct WireguardStatus {
    pub interface_up: bool,
    pub listening_port: u16,
    pub peers: Vec<WireguardPeer>,
}

#[derive(Debug, Serialize)]
pub struct WireguardPeer {
    pub name: String,
    pub public_key: String,
    pub last_handshake: Option<String>,
    pub transfer_rx: u64,
    pub transfer_tx: u64,
}
```

### CrowdSec Decisions (shannon sec blocks)

```rust
#[derive(Debug, Serialize)]
pub struct CrowdsecDecision {
    pub id: u64,
    pub source_ip: String,
    pub reason: String,
    pub action: String,  // "ban"
    pub duration: String,
    pub origin: String,  // "crowdsec" or "cscli"
}
```

---

## API Design

### AdGuard Home API

AdGuard exposes REST API on `:3000` (or custom port):

```bash
# Status
GET http://localhost:3000/control/status
→ {"dns_addresses":["192.168.4.1:53"],"dns_port":53,"http_port":3000,...}

# Stats
GET http://localhost:3000/control/stats
→ {"time_units":"hours","num_dns_queries":1234,"num_blocked_filtering":567,...}
```

### CrowdSec CLI Wrapper

CrowdSec's `cscli` is the canonical interface:

```bash
# Status
cscli metrics
→ Acquisition Metrics, Parser Metrics, Scenario Metrics...

# Decisions
cscli decisions list -o json
→ [{"id":1,"origin":"crowdsec","scope":"Ip","value":"1.2.3.4","reason":"ssh-bf",...}]

# Whitelist
cscli decisions delete --ip 1.2.3.4
```

### WireGuard CLI Wrapper

Native `wg` command:

```bash
# Status
wg show wg0
→ interface: wg0
→   public key: <server-pubkey>
→   private key: (hidden)
→   listening port: 51820
→ peer: <peer-pubkey>
→   allowed ips: 10.0.0.2/32
→   latest handshake: 2 minutes, 14 seconds ago
→   transfer: 1.23 GiB received, 456.78 MiB sent

# Generate keys
wg genkey | tee privatekey | wg pubkey > publickey
```

---

## Installation Sequence

### Step 1: AdGuard Home

```bash
# Download and install
curl -s -S -L https://raw.githubusercontent.com/AdguardTeam/AdGuardHome/master/scripts/install.sh | sh -s -- -v

# Initial setup via web UI at :3000
# Set DNS to listen on 192.168.4.1:53
# Configure DoH upstream: https://dns.cloudflare.com/dns-query
# Enable OISD blocklist

# Disable dnsmasq DNS
sed -i 's/^#port=.*/port=0/' /etc/dnsmasq.conf
echo "dhcp-option=6,192.168.4.1" >> /etc/dnsmasq.conf
systemctl restart dnsmasq
```

### Step 2: CrowdSec

```bash
# Install engine
curl -s https://install.crowdsec.net | sudo sh
apt install crowdsec

# Install nftables bouncer
apt install crowdsec-firewall-bouncer-nftables

# Configure log acquisition
cat > /etc/crowdsec/acquis.yaml <<EOF
filenames:
  - /var/log/auth.log
labels:
  type: syslog
---
filenames:
  - /var/log/syslog
labels:
  type: syslog
EOF

# Enable community blocklist
cscli hub update
cscli collections install crowdsecurity/linux
cscli collections install crowdsecurity/sshd

systemctl enable crowdsec
systemctl start crowdsec
```

### Step 3: WireGuard

```bash
# Install userspace tools (kernel module already present in 6.18)
apt install wireguard-tools qrencode

# Generate server keys
cd /etc/wireguard
wg genkey | tee server_private | wg pubkey > server_public
chmod 600 server_private

# Create server config
cat > wg0.conf <<EOF
[Interface]
Address = 10.0.0.1/24
ListenPort = 51820
PrivateKey = $(cat server_private)

# iPhone
[Peer]
PublicKey = <iphone-pubkey>
AllowedIPs = 10.0.0.2/32

# MacBook Pro
[Peer]
PublicKey = <mbp-pubkey>
AllowedIPs = 10.0.0.3/32

# Mac Mini
[Peer]
PublicKey = <macmini-pubkey>
AllowedIPs = 10.0.0.4/32
EOF

# Enable and start
systemctl enable wg-quick@wg0
systemctl start wg-quick@wg0

# Generate client configs with QR codes
# (See tasks.md for peer generation script)
```

---

## Notification Integration

### CrowdSec → ntfy-bridge → TTS Daemon

**Existing infrastructure** (no new webhook server needed):
- `ntfy-bridge` (`~/Projects/ntfy-bridge`) — Rust daemon on Mac, subscribes to ntfy topics on Dell (192.168.4.84:8099), forwards to TTS daemon socket (`/tmp/claude-tts-daemon.sock`)
- TTS daemon already supports notification levels (urgent → TTS/Ruby, medium → ntfy, routine → log)

**Integration approach:**
1. CrowdSec notification plugin on SHANNON POSTs alerts to ntfy server on Dell
2. ntfy-bridge (Mac) receives and forwards to TTS daemon socket
3. Severity-based routing: repeated bans → urgent topic (TTS/Ruby); single bans → normal topic (ntfy); routine → log only

```
SHANNON                          Dell                    Mac Mini
CrowdSec → ntfy POST ────────→ ntfy server ──────────→ ntfy-bridge
                                (:8099)                  ↓
                                                     TTS daemon socket
                                                         ↓
                                                     Ruby narrates
```

---

## LLM Security Analysis (Dual-Layer)

### Architecture

CrowdSec and GPT-5-nano serve complementary roles:
- **CrowdSec**: Real-time pattern-based IDS. Community blocklist, behavioral scenarios, nftables bouncer. Catches known attack patterns instantly.
- **GPT-5-nano**: Semantic reasoning layer. Hourly batch analysis of raw logs + CrowdSec decisions. Catches novel patterns, multi-step correlation, behavioral anomalies that signatures miss.

Inspired by Anthropic's Claude Code Security (Observe→Hypothesize→Test→Refine→Verify loops, 500+ zero-days found). CrowdSec is the signature engine; the LLM is the reasoning engine.

### Model Selection (Two-Tier, ~$3/month)

**Hourly triage: GPT-5-nano** ($0.05/M input, $0.40/M output)
- Fast anomaly sweep on last hour's logs (~10K tokens)
- ~$0.001/run × 720 runs/month = ~$0.72/month
- Flags obvious anomalies; anything ambiguous gets picked up by daily Gemini
- **Critical**: GPT-5-nano is a reasoning model — most tokens go to internal `reasoning_tokens`, not visible output. Use `max_completion_tokens: 2000` (not 300). Does NOT support custom `temperature` or legacy `max_tokens` parameter.

**Daily deep analysis: Gemini 3.1 Pro** (active, billing enabled Feb 2026)
- Full day's logs + CrowdSec decisions + hourly triage flags in single call (~20-50K tokens)
- Pattern correlation, behavioral anomaly detection, multi-step reasoning
- Model configurable via `GEMINI_MODEL` env var in `/etc/shannon-security/env`
- **Note**: Free tier has quota=0 for all Pro models. Billing required (~$2/month).
- **Cost**: ~$0.07/run × 30 runs/month = ~$2.10/month

**Total: ~$2.82/month** (budget: $5/month, requires billing for Pro)

### Hourly Triage Flow (GPT-5-nano)

```
Cron (hourly) on SHANNON:
  1. Collect: tail -1000 auth.log, syslog, cscli decisions list --since 1h
  2. Send to GPT-5-nano with quick triage prompt
  3. Categorize: critical / normal / clear
  4. Route findings:
     - Critical → ntfy urgent topic → TTS/Ruby immediate
     - Normal → append to daily digest file
     - Clear → log only
```

### Daily Deep Analysis Flow (Gemini)

```
Cron (daily, 06:00) on SHANNON:
  1. Collect: last 24h auth.log, syslog, cscli decisions, hourly triage flags
  2. Send to Gemini (configurable model) with deep analysis prompt
  3. Produce: pattern correlation, behavioral anomalies, trend detection
  4. Route: digest → ntfy normal topic → logged for review
  5. Generate escalation prompts for anything actionable
```

### Data Model

```rust
#[derive(Debug, Serialize)]
pub struct LlmSecurityDigest {
    pub timestamp: String,
    pub logs_analyzed: u32,
    pub crowdsec_decisions_reviewed: u32,
    pub findings: Vec<SecurityFinding>,
    pub escalation_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SecurityFinding {
    pub severity: Severity,  // Critical, Medium, Low
    pub category: String,    // "novel_pattern", "multi_step", "behavioral"
    pub description: String,
    pub evidence: Vec<String>,
    pub recommendation: String,
}
```

### Escalation to Claude Code

For significant alerts, generate a copyable prompt:
```
"CrowdSec Alert: IP 185.x.x.x blocked for ssh-bf (10 attempts in 5 minutes).
This IP has been blocked by CrowdSec. To investigate:
- ssh shannon 'cscli decisions list --ip 185.x.x.x'
- ssh shannon 'journalctl -u ssh --since \"1 hour ago\" | grep 185.x.x.x'

Paste this into Claude Code to analyze the attack pattern and decide on permanent blocking."
```

---

## Trade-off Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| DNS provider | AdGuard Home | Clean dnsmasq split, single binary, built-in DoH/DoT |
| IDS approach | CrowdSec | Crowd intelligence, ARM64 support, lighter than Suricata |
| VPN protocol | WireGuard | In-kernel, zero RAM, fastest |
| CLI depth | Status + basic ops | Native tools handle advanced cases |
| Key rotation | Manual | 4 peers, you control the devices |
| Monitoring | Existing TTS daemon | No new monitoring infra |

---

## Security Considerations

- **AdGuard admin**: Password-protect web UI, bind to localhost or LAN only
- **CrowdSec API**: Local only, no external exposure
- **WireGuard keys**: Stored in `/etc/wireguard/` with 600 permissions
- **SSH hardened** (Feb 23, 2026): Password auth disabled, listen restricted to LAN (192.168.4.1) + VPN (10.0.0.1) only. Public internet cannot reach SSH. Config: `/etc/ssh/sshd_config.d/hardening.conf`

---

## Resource Budget

| Component | Idle RAM | Peak RAM |
|-----------|----------|----------|
| AdGuard Home | ~80 MB | ~200 MB |
| CrowdSec engine + bouncer | ~150 MB | ~300 MB |
| WireGuard | ~0 MB | ~0 MB |
| **Total added** | **~230 MB** | **~500 MB** |
| **Available (4 GB)** | ~3.7 GB | ~3.5 GB |
