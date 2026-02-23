# shannon

Unified router management CLI for Rock Pi 4B SE (SHANNON).

A token-efficient CLI designed for both humans and AI agents to manage DNS, DHCP, firewall, VPN, and security on a home router.

**v0.1.0** deployed on SHANNON at `/usr/local/bin/shannon`. Security stack (AdGuard Home + CrowdSec + WireGuard) installed and operational.

## Features

- **System Health**: `shannon status` overview, `shannon doctor` diagnostics
- **DNS Management**: Add/remove/list local DNS records
- **DHCP Management**: View leases, add static reservations
- **Firewall**: Port forwarding, IP blocking
- **Security Stack**: CrowdSec IDS, AdGuard Home DNS filtering, WireGuard VPN
- **LLM Security Analysis**: GPT-5-nano hourly triage + Gemini 3.1 Pro daily deep analysis (~$3/month)
- **Dynamic DNS**: Auto-updates `shannon.fredrikbranstrom.se` via Loopia API on IP change
- **Dual Output**: Plain text for humans, `--json` for AI agents
- **Location-Aware**: Works locally on SHANNON or remotely via SSH

## Installation

### From Source

```bash
# Build for current platform
cargo build --release

# Copy to SHANNON
scp target/release/shannon shannon:/usr/local/bin/

# Or use cross-compilation for ARM64
cross build --release --target aarch64-unknown-linux-gnu
scp target/aarch64-unknown-linux-gnu/release/shannon shannon:/usr/local/bin/
```

### Mac Alias (Optional)

Add to your `~/.zshrc`:
```bash
alias shannon='ssh shannon shannon'
```

## Usage

```bash
# System health
shannon status              # Overview (WAN IP, memory, services)
shannon doctor              # Run diagnostic checks

# DNS management
shannon dns list            # List all DNS records
shannon dns add myhost 192.168.4.100
shannon dns rm myhost

# DHCP management
shannon dhcp leases         # List all leases
shannon dhcp reserve aa:bb:cc:dd:ee:ff 192.168.4.100 --hostname mydevice
shannon dhcp unreserve aa:bb:cc:dd:ee:ff

# Firewall
shannon fw list             # List port forwards
shannon fw forward 8080 192.168.4.84:80 --proto tcp
shannon fw unforward 8080
shannon fw block 1.2.3.4
shannon fw unblock 1.2.3.4

# Security
shannon sec status          # Health of AdGuard, CrowdSec, WireGuard
shannon sec blocks          # Active CrowdSec decisions (blocked IPs)
shannon sec scan            # Run security analysis
shannon sec report          # View recent findings

# VPN
shannon vpn peers           # WireGuard peers with handshake status
shannon vpn status          # WireGuard interface status

# Dynamic DNS
shannon ddns status         # WAN IP, DNS record, timer status
shannon ddns update         # Check and update if IP changed
shannon ddns update --force # Force DNS update
```

### AI Agent Usage

All commands support `--json` for structured output:

```bash
shannon status --json
shannon dhcp leases --json
```

Use `--yes` to skip confirmation prompts for automation:

```bash
shannon fw forward 8080 192.168.4.84:80 --yes
```

## Architecture

```
shannon CLI
├── status         → sysinfo + systemctl (5 services)
├── doctor         → diagnostic checks (top-level)
├── dns            → dnsmasq config parsing
├── dhcp           → dnsmasq leases + dhcp-host
├── fw             → nftables rules
├── sec            → CrowdSec + AdGuard adapters
│   ├── status     → combined health (AdGuard + CrowdSec + WireGuard)
│   └── blocks     → active CrowdSec decisions
├── vpn            → WireGuard adapter
│   ├── peers      → peer list with handshake status
│   └── status     → interface overview
└── ddns           → Dynamic DNS (Loopia API)
    ├── status     → WAN IP, DNS record, timer
    └── update     → check and update if changed
```

## Security Stack

Deployed and running. Specs at `.claude/specs/security-stack/`. RAM: ~438 MB total (11.4% of 4 GB).

### Components

| Service | Port | RAM | Purpose |
|---------|------|-----|---------|
| AdGuard Home | 53 (DNS), 3000 (web) | ~105 MB | DNS filtering, 507K+ blocklist rules (OISD + AdGuard default), DoH upstream (Cloudflare + Quad9) |
| CrowdSec | — | ~88 MB | IDS with nftables bouncer, SSH monitoring, community blocklist |
| WireGuard | 51820 | ~0 MB | VPN (kernel-space), 4 peers configured |

dnsmasq remains for DHCP only (`port=0`). All DNS queries go through AdGuard Home.

### LLM Security Analysis (~$3/month)

Dual-layer approach complementing CrowdSec's pattern matching with LLM semantic reasoning:

- **Hourly**: GPT-5-nano triage (`/usr/local/bin/shannon-triage-hourly`) — categorizes critical/normal/clear
- **Daily**: Gemini 3.1 Pro deep analysis (`/usr/local/bin/shannon-triage-daily`) — pattern correlation, behavioral anomalies, trend detection

Results saved to `/var/log/shannon-security-analyses/`. Critical findings route to ntfy urgent topic.

### Alert Pipeline

SHANNON CrowdSec → ntfy server (Dell) → ntfy-bridge (Mac) → TTS daemon → Ruby narrates.

**Note**: Dell currently unreachable at 192.168.4.84 (needs network config update post-Huddinge move).

### Dynamic DNS

`shannon.fredrikbranstrom.se` auto-updated via Loopia XMLRPC API. Python script reads WAN IP directly from interface (zero external calls), updates DNS only on change. systemd timer every 5 min.

| Component | Path |
|-----------|------|
| Update script | `/usr/local/lib/shannon-security/ddns_update.py` |
| Symlink | `/usr/local/bin/shannon-ddns` |
| Timer | `shannon-ddns.timer` (5 min) |
| State | `/var/cache/shannon-ddns-state.json` |
| Credentials | `/etc/shannon-security/env` (`LOOPIA_USER`, `LOOPIA_PASSWORD`) |

**WAN interface**: `enxc84d4421f975` (USB ethernet, DHCP lease from Bahnhof). IP is dynamic — DDNS essential for WireGuard endpoint stability.

## Configuration

Default paths (on SHANNON):
- dnsmasq config: `/etc/dnsmasq.conf`
- Custom DNS: `/etc/dnsmasq.d/custom.conf`
- DHCP leases: `/var/lib/misc/dnsmasq.leases`
- SSH hardening: `/etc/ssh/sshd_config.d/hardening.conf` (key-only, LAN+VPN listen)
- nftables rules: managed by CrowdSec bouncer + WireGuard PostUp/PostDown
- WireGuard: `/etc/wireguard/wg0.conf`, peer configs in `/etc/wireguard/peers/`
- AdGuard Home: `/opt/AdGuardHome/AdGuardHome.yaml`
- CrowdSec: `/etc/crowdsec/config.yaml`, notifications in `/etc/crowdsec/notifications/`
- LLM scripts: `/usr/local/lib/shannon-security/`, API keys in `/etc/shannon-security/env`
- LLM logs: `/var/log/shannon-llm-triage.log`, analyses in `/var/log/shannon-security-analyses/`

## Requirements

- SHANNON running Armbian Trixie (kernel 6.18, ARM64) with dnsmasq
- SSH access configured (`~/.ssh/config` with `Host shannon`)
- Root access on SHANNON
- Public IP on WAN interface (no port forwarding needed — SHANNON is the border device)

## License

MIT

## Author

Fredrik Bränström
