# shannon

Unified router management CLI for Rock Pi 4B SE (SHANNON).

A token-efficient CLI designed for both humans and AI agents to manage DNS, DHCP, firewall, and security on a home router.

## Features

- **System Health**: `shannon status` and `shannon doctor` for diagnostics
- **DNS Management**: Add/remove/list local DNS records
- **DHCP Management**: View leases, add static reservations
- **Firewall**: Port forwarding, IP blocking
- **Security Analysis**: LLM-based log analysis (coming soon)
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
shannon status doctor       # Run diagnostic checks

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

# Security (coming soon)
shannon sec scan            # Run security analysis
shannon sec report          # View recent findings
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
├── status/doctor  → sysinfo + systemctl
├── dns            → dnsmasq config parsing
├── dhcp           → dnsmasq leases + dhcp-host
├── fw             → iptables/nftables
└── sec            → GPT-5-mini log analysis (TODO)
```

## Configuration

Default paths (on SHANNON):
- dnsmasq config: `/etc/dnsmasq.conf`
- Custom DNS: `/etc/dnsmasq.d/custom.conf`
- DHCP leases: `/var/lib/misc/dnsmasq.leases`
- iptables rules: `/etc/iptables/rules.v4`

## Requirements

- SHANNON running Armbian with dnsmasq
- SSH access configured (`~/.ssh/config` with `Host shannon`)
- Root or sudo access on SHANNON

## License

MIT

## Author

Fredrik Bränström
