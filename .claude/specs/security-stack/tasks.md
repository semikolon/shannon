# Tasks: SHANNON Security Stack (Phase 2)

## Overview

- **Estimated scope**: Medium (4-6 hours installation + CLI integration)
- **Priority order**: CrowdSec first (user preference), then AdGuard Home, then WireGuard
- **Cut first if needed**: CLI integration (can use native tools)
- **Rollback safety**: SSH on LAN (192.168.4.1) always reachable from local network

---

## Phase 1: Pre-Installation Verification

- [x] **T-1**: Verify SHANNON connectivity and baseline state
  - CLI works, 268 MB RAM, 3.5 GB free, dnsmasq healthy

- [x] **T-2**: Verify package availability
  - CrowdSec + bouncer in apt, WireGuard kernel module loaded

- [x] **T-3**: Document current dnsmasq config for rollback
  - Saved to `/tmp/dnsmasq.conf.before` on Mac

---

## Phase 2: CrowdSec Installation (Priority 1)

- [x] **T-4**: Install CrowdSec engine
  - Installed from Debian repos (v1.4.6)

- [x] **T-5**: Configure log acquisition
  - Already configured: auth.log, syslog, SSH journal

- [x] **T-6**: Install collections and scenarios
  - `crowdsecurity/linux` + `crowdsecurity/sshd` with GeoIP databases

- [x] **T-7**: Install nftables bouncer
  - Required CrowdSec's own repo (not in Debian). Installed v0.0.34.

- [x] **T-8**: Verify CrowdSec operation
  - Active and blocking. Already banned 2 IPs for SSH brute-force on first boot.
  - **Gotcha**: Plugin binaries in `/usr/lib/crowdsec/plugins/` needed renaming to `notification-{type}` format (Debian packaging bug).

- [ ] **T-9**: Test CrowdSec detection (optional - safe to skip)

---

## Phase 3: AdGuard Home Installation

- [x] **T-10**: Download and install AdGuard Home
  - Installed v0.107.72 via official script

- [x] **T-11**: Initial setup via API
  - Admin: `admin` / DNS on 192.168.4.1:53
  - DoH upstreams: Cloudflare (`https://dns.cloudflare.com/dns-query`) + Quad9 (`https://dns.quad9.net/dns-query`)

- [x] **T-12**: Enable blocklists
  - OISD (358K rules) + AdGuard default (149K rules) = 507K+ rules

- [x] **T-13**: Disable dnsmasq DNS
  - Added `port=0` to `/etc/dnsmasq.conf`
  - DNS server option already existed at line 28 (`dhcp-option=option:dns-server,192.168.4.1`)
  - Fixed `/etc/resolv.conf` to point to 192.168.4.1 (was 127.0.0.1)
  - Installed `dnsutils` for `dig`
  - **Gotcha**: Adding `dhcp-option=6,192.168.4.1` created a duplicate — option 6 IS `dns-server`. Fixed Feb 23 by commenting out the duplicate.

- [x] **T-14**: Verify DNS cutover
  - google.com resolves normally, doubleclick.net blocked (0.0.0.0)

---

## Phase 4: WireGuard Installation

- [x] **T-15**: Install WireGuard tools
  - `wireguard-tools` + `qrencode`

- [x] **T-16**: Generate server keys
  - `/etc/wireguard/server_private` (chmod 600), `server_public`

- [x] **T-17**: Create server config
  - `wg0.conf` with NAT masquerade (PostUp/PostDown nft rules for `end0`)

- [x] **T-18**: Generate peer keys (4 peers)
  - iPhone (10.0.0.2), MBP (10.0.0.3), Mac Mini (10.0.0.4), Future (10.0.0.5)
  - All keys + client configs in `/etc/wireguard/peers/`
  - Endpoint set to 94.254.88.116:51820

- [x] **T-19**: Start WireGuard
  - `wg-quick@wg0` enabled and running. Zero RAM overhead (kernel-space).

- [x] **T-20**: ~~Configure port forwarding~~ NOT NEEDED
  - SHANNON is the border device with public IP directly on WAN interface.
  - WireGuard listens on all interfaces, nftables INPUT policy is accept.
  - No Deco/ONT forwarding required.

- [ ] **T-21**: Test VPN from mobile device
  - Generate QR: `qrencode -t ansiutf8 < /etc/wireguard/peers/iphone.conf`
  - Import in WireGuard app, test on mobile data
  - **User action required**
  - **Priority**: HIGH — SSH now restricted to LAN+VPN, VPN must be verified

---

## Phase 5: CLI Integration (shannon)

- [x] **T-22**: Add WireGuard adapter
  - `src/adapters/wireguard.rs` — parses `wg show`, resolves peer names from wg0.conf comments
  - **Gotcha**: Base64 keys contain `=`; `split('=').nth(1)` truncates. Use `find('=')` + slice instead.

- [x] **T-23**: Add CrowdSec adapter
  - `src/adapters/crowdsec.rs` — wraps `cscli decisions list -o json`, `systemctl is-active`

- [x] **T-24**: Add AdGuard adapter
  - `src/adapters/adguard.rs` — REST API calls to localhost:3000/control/{stats,filtering/status}

- [x] **T-25**: Extend sec command with status
  - `shannon sec status` shows combined SecurityStatus (AdGuard + CrowdSec + WireGuard)

- [x] **T-26**: Add sec blocks subcommand
  - `shannon sec blocks` lists active CrowdSec decisions

- [x] **T-27**: Add vpn command
  - `src/commands/vpn.rs` — `vpn peers` and `vpn status`

- [x] **T-28**: Update CLI definitions
  - VpnAction + SecAction enums in cli.rs, wired in main.rs

- [x] **T-29**: Build and deploy updated shannon
  - Built on SHANNON (ARM64, `cargo build --release`, ~72s)
  - **Note**: `source /root/.cargo/env` needed for non-interactive SSH (cargo not in default PATH)

- [x] **T-30**: Test CLI integration
  - All commands verified: `sec status`, `sec status --json`, `sec blocks`, `vpn peers`
  - `shannon status` now shows all 5 services (dnsmasq, ssh, crowdsec, AdGuardHome, wg-quick@wg0)

---

## Phase 6: Notification Integration

- [x] **T-31**: Configure CrowdSec notification plugin to POST to ntfy
  - Created `/etc/crowdsec/notifications/http.yaml` (POST to `http://192.168.4.84:8099/shannon-security`)
  - Enabled `http_default` in `profiles.yaml`
  - **Blocked**: Dell not reachable at 192.168.4.84 (ARP FAILED — likely still has old Deco-era IP config)

- [ ] **T-32**: Verify ntfy-bridge receives and forwards alerts
  - Depends on Dell being reachable

- [ ] **T-33**: Create escalation prompt template
  - Deferred until notification pipeline verified

---

## Phase 6.5: LLM Security Analysis (Two-Tier, ~$3/month)

- [x] **T-33a**: Create log collection script on SHANNON
  - `/usr/local/lib/shannon-security/collect_logs.sh`
  - Collects: journalctl (SSH, syslog warnings), cscli decisions/alerts, network stats, AdGuard stats
  - Output: structured JSON, parameterized `--since 1h|24h`

- [x] **T-33b**: Create hourly triage (GPT-5-nano)
  - `/usr/local/lib/shannon-security/hourly_triage.sh` → `/usr/local/bin/shannon-triage-hourly`
  - Categorizes: critical → ntfy urgent, normal → daily digest, clear → log only
  - **Gotcha**: Symlink SCRIPT_DIR resolves to `/usr/local/bin/`, not source dir. Use `readlink -f "$0"`.
  - **Gotcha**: GPT-5-nano is a reasoning model — `max_completion_tokens: 300` leaves nothing for output (all goes to `reasoning_tokens`). Need 2000+.
  - **Gotcha**: GPT-5-nano doesn't support `max_tokens` (use `max_completion_tokens`) or custom `temperature` (only default 1).

- [x] **T-33c**: Create daily deep analysis (Gemini)
  - `/usr/local/lib/shannon-security/daily_analysis.sh` → `/usr/local/bin/shannon-triage-daily`
  - Pattern correlation, behavioral anomalies, trend detection, escalation prompts
  - Saves analyses to `/var/log/shannon-security-analyses/YYYY-MM-DD.json`
  - **Model**: Configurable via `GEMINI_MODEL` env var. Set to `gemini-3.1-pro-preview` (billing enabled Feb 22, 2026).
  - **Gotcha**: Gemini Pro models (3, 3.1) have free tier quota = 0. Requires Google AI billing (~$2/month).
  - **Gotcha**: `maxOutputTokens: 2000` insufficient for thinking models. Use 8192+.
  - **Gotcha**: Shell argument limits hit at ~130KB. Use temp files + `jq --rawfile` + `curl -d @file`.

- [x] **T-33d**: Set up cron jobs on SHANNON
  - Hourly: `0 * * * *` → `shannon-triage-hourly`
  - Daily: `0 6 * * *` → `shannon-triage-daily`
  - Log: `/var/log/shannon-llm-triage.log`
  - API keys deployed to `/etc/shannon-security/env` (chmod 600)

- [x] **T-33e**: Verify two-tier operation
  - Hourly triage verified: category=normal, SSH brute-force detected and mitigated, legitimate admin access confirmed
  - First entry in daily digest: `/var/log/shannon-daily-digest.jsonl`
  - Daily analysis verified with Gemini 3.1 Pro (Feb 23): severity yellow, CrowdSec crash + auto-recovery correctly identified

- [x] **T-33f**: Enrich log collection with system state context
  - Added to `collect_logs.sh`: service states (active/failed + `active_since`), systemd lifecycle events (Started/Stopped/Failed), system resources (memory, load, temp, disk)
  - Purpose: LLM can now distinguish "crashed and down" from "crashed but self-healed" — first Gemini analysis without this context over-reported CrowdSec crash severity

---

## Phase 6.7: Dynamic DNS (Loopia API)

- [x] **T-40**: Create DDNS update script
  - `/usr/local/lib/shannon-security/ddns_update.py` → `/usr/local/bin/shannon-ddns`
  - Python 3.13 + built-in `xmlrpc.client` (zero pip dependencies)
  - Reads WAN IP from `enxc84d4421f975` interface directly (no external HTTP calls)
  - Updates `shannon.fredrikbranstrom.se` A record via Loopia XMLRPC API (TTL=300)
  - Cache-based comparison: only calls API on IP change
  - ntfy notification on IP change (when Dell reachable)
  - `--status` flag for quick state check, `--force` for manual override

- [x] **T-41**: Deploy systemd timer
  - `shannon-ddns.timer` / `shannon-ddns.service`
  - Runs every 5 min with 30s randomized delay, starts 30s after boot
  - State persisted to `/var/cache/shannon-ddns-state.json`
  - **Currently disabled** (Feb 23): Timer stopped to prevent log spam — no Loopia credentials yet. Re-enable: `systemctl enable --now shannon-ddns.timer`

- [x] **T-42**: Add CLI integration
  - `shannon ddns status` — reads state file, shows WAN IP, DNS record, timer status
  - `shannon ddns update [--force]` — wraps Python script, shows result
  - JSON output supported (`--json`)

- [ ] **T-43**: Add Loopia API credentials
  - Needs `LOOPIA_USER` and `LOOPIA_PASSWORD` in `/etc/shannon-security/env`
  - Same API user as Dell's certbot (`/etc/letsencrypt/loopia.ini`)
  - Also needs `addSubdomain` + `updateZoneRecord` permissions
  - **Blocked**: Dell unreachable, credentials only stored there
  - **User action**: Either access Dell or log into Loopia Customer Zone directly
  - After adding: `systemctl enable --now shannon-ddns.timer`

- [ ] **T-44**: Update WireGuard peer configs
  - After DDNS is working, change endpoint from `94.254.88.116:51820` to `shannon.fredrikbranstrom.se:51820`
  - Update all 4 peer configs in `/etc/wireguard/peers/`
  - Regenerate QR codes for mobile devices

---

## Phase 6.8: SSH Hardening

- [x] **T-45**: Disable password authentication
  - Root had a password set (`passwd -S root` → `P`), password auth was enabled by default
  - Zero password logins ever occurred (verified via full journal scan)
  - Created `/etc/ssh/sshd_config.d/hardening.conf`: `PasswordAuthentication no`, `KbdInteractiveAuthentication no`

- [x] **T-46**: Restrict SSH to LAN + VPN only
  - Added `ListenAddress 192.168.4.1` + `ListenAddress 10.0.0.1` to hardening.conf
  - SSH no longer reachable from public internet — eliminates all brute-force noise
  - LAN IP is static (netplan `10-router.yaml`), no risk of DHCP reassignment
  - **Prerequisite for remote access**: VPN must work (T-21)

---

## Phase 7: Configuration Git Tracking

- [x] **T-47**: Track security scripts in shannon repo
  - All scripts from `/usr/local/lib/shannon-security/` committed to `scripts/security/`
  - Systemd units for DDNS committed
  - CrowdSec notification config committed to `configs/`
  - SSH hardening config committed to `configs/`
  - Checksums verified: repo is byte-identical to deployed versions

- [ ] **T-34**: Create private repo for SHANNON system configs
  - For sensitive configs not suitable for public repo (wg keys, full system configs)
  - Or: use encrypted secrets in existing repo

- [ ] **T-35**: Set up config sync workflow
  - Script or Makefile to deploy from repo to SHANNON

- [ ] **T-36**: Document restore procedure
  - From-scratch rebuild guide using repo as source

---

## Phase 8: Documentation

- [x] **T-37**: Update shannon README.md
  - Security stack, LLM analysis, DDNS, SSH hardening, architecture diagram all current

- [x] **T-38**: Update SHANNON section in dotfiles docs
  - SYSTEM_REFERENCE.md: DNS table, security stack status (installed and operational), stale Dell IP noted, GPT-5-nano (not mini), Gemini 3.1 Pro

- [ ] **T-39**: Create peer onboarding guide

---

## Phase 9: DNS Cleanup (blocked on Dell)

- [ ] **T-48**: Update `*.fredrikbranstrom.se` wildcard DNS
  - Currently points to `213.164.219.201` (old Huddinge/Dell IP, stale)
  - All subdomains (brf-auto, blobulator, registry) resolve to dead IP
  - Need Dell's new Sarpetorp IP, or remove wildcard and set explicit records
  - **Blocked**: Dell unreachable

- [ ] **T-49**: Verify Dell network config
  - Dell likely still has Deco-era IP config from Huddinge
  - Needs manual check — power cycle or console access
  - Unblocks: T-32 (ntfy), T-33 (escalation), T-43 (Loopia creds), T-48 (DNS wildcard)

---

## Verification Checklist

Before marking Phase 2 complete:

- [ ] **V-1**: Ads blocked on all network devices (YouTube, web)
- [x] **V-2**: `cscli metrics` shows active scenarios — scenarios loaded, 4 alerts in first 24h
- [ ] **V-3**: `cscli decisions list` shows community blocklist entries — bans active but short TTL, currently 0 active (normal for low traffic)
- [ ] **V-4**: WireGuard connects from mobile data
- [ ] **V-5**: `ssh shannon` works over VPN tunnel
- [x] **V-6**: DNS resolution works (AdGuard serving) — verified with dig
- [x] **V-7**: DHCP still working (dnsmasq) — 18 active leases
- [x] **V-8**: `shannon sec status` shows all green — all 3 services active
- [x] **V-9**: RAM usage <1 GB total — 449 MB (12%)
- [x] **V-10**: Configs committed to git — scripts + configs in shannon repo, checksums verified

---

## Implementation Notes

- **RAM budget**: 449 MB total (CrowdSec ~88 MB, AdGuard ~105 MB, WireGuard ~0 MB). Well under 1 GB target.
- **No port forwarding needed**: SHANNON has public IP directly on WAN interface. Spec assumption about Deco/ONT forwarding was wrong.
- **Dell unreachable**: 192.168.4.84 (LAN) and 213.164.219.201 (old public IP) both unreachable from SHANNON. Likely still has Deco-era network config. Blocks: ntfy notifications, Loopia credentials, DNS wildcard update.
- **CrowdSec plugin naming**: Debian-packaged plugins in `/usr/lib/crowdsec/plugins/` named `http` instead of `notification-http`. Required manual rename.
- **CrowdSec crash** (Feb 22 16:03): Crashed once, auto-restarted via systemd at 16:04. Root cause unknown. Stable since (17h+). Monitor.
- **Cargo PATH on SHANNON**: Non-interactive SSH doesn't source `.cargo/env`. Use `source /root/.cargo/env && cargo build` for builds.
- **WireGuard key parsing**: Base64 keys end with `=` — can't use `split('=')` for config parsing. Use `find('=')` position + slice.
- **SSH hardened** (Feb 23): Password auth disabled + listen restricted to LAN+VPN only. Root had a password set but zero password logins ever occurred. Config: `/etc/ssh/sshd_config.d/hardening.conf`.
- **Gemini billing** (Feb 23): Google AI billing enabled via Google Cloud Console (hard to find). `GEMINI_MODEL=gemini-3.1-pro-preview` active. First analysis ran successfully.
- **Log collection enriched** (Feb 23): `collect_logs.sh` now includes service states, restart history, system resources (RAM/load/temp/disk). Gives LLM context to distinguish crashes from self-healing.
- **DDNS timer disabled** (Feb 23): Stopped to prevent log spam (98 failures/day). Re-enable after adding Loopia credentials (T-43).
- **Duplicate dhcp-option 6** (Feb 23): `dhcp-option=option:dns-server` (line 28) and `dhcp-option=6` (line 46) are the same thing. Duplicate commented out. Warning eliminated.
- **Port 2222 forward**: nftables still has `WAN:2222 → 192.168.4.84:22` rule (Dell SSH forward from pre-Sarpetorp). Harmless since Dell is unreachable, but should be cleaned up when Dell surfaces.
