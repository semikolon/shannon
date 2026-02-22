# Tasks: SHANNON Security Stack (Phase 2)

## Overview

- **Estimated scope**: Medium (4-6 hours installation + CLI integration)
- **Priority order**: CrowdSec first (user preference), then AdGuard Home, then WireGuard
- **Cut first if needed**: CLI integration (can use native tools)
- **Rollback safety**: IP-based SSH (192.168.4.1) survives all DNS changes

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
  - Added `port=0` and `dhcp-option=6,192.168.4.1` to `/etc/dnsmasq.conf`
  - Fixed `/etc/resolv.conf` to point to 192.168.4.1 (was 127.0.0.1)
  - Installed `dnsutils` for `dig`

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
  - **Model**: Configurable via `GEMINI_MODEL` env var. Default `gemini-2.5-flash` (free tier). Set to `gemini-3.1-pro-preview` after enabling billing.
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
  - Daily analysis pending next 06:00 UTC run

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

- [ ] **T-44**: Update WireGuard peer configs
  - After DDNS is working, change endpoint from `94.254.88.116:51820` to `shannon.fredrikbranstrom.se:51820`
  - Update all 4 peer configs in `/etc/wireguard/peers/`
  - Regenerate QR codes for mobile devices

---

## Phase 7: Configuration Git Tracking

- [ ] **T-34**: Create private repo for SHANNON configs
- [ ] **T-35**: Set up config sync
- [ ] **T-36**: Document restore procedure

---

## Phase 8: Documentation

- [x] **T-37**: Update shannon README.md
  - Security stack, LLM analysis, DDNS, architecture diagram all current
- [x] **T-38**: Update SHANNON section in dotfiles docs
  - SYSTEM_REFERENCE.md DNS table updated with `shannon` DDNS record, stale Dell IP noted
- [ ] **T-39**: Create peer onboarding guide

---

## Verification Checklist

Before marking Phase 2 complete:

- [ ] **V-1**: Ads blocked on all network devices (YouTube, web)
- [ ] **V-2**: `cscli metrics` shows active scenarios
- [ ] **V-3**: `cscli decisions list` shows community blocklist entries
- [ ] **V-4**: WireGuard connects from mobile data
- [ ] **V-5**: `ssh shannon` works over VPN tunnel
- [x] **V-6**: DNS resolution works (AdGuard serving) — verified with dig
- [x] **V-7**: DHCP still working (dnsmasq) — 18 active leases
- [x] **V-8**: `shannon sec status` shows all green — all 3 services active
- [x] **V-9**: RAM usage <1 GB total — 438 MB (11.4%)
- [ ] **V-10**: Configs committed to git

---

## Implementation Notes

- **RAM budget**: 438 MB total (CrowdSec ~88 MB, AdGuard ~105 MB, WireGuard ~0 MB). Well under 1 GB target.
- **No port forwarding needed**: SHANNON has public IP directly on WAN interface. Spec assumption about Deco/ONT forwarding was wrong.
- **Dell unreachable**: 192.168.4.84 returns ARP FAILED from SHANNON. Dell likely still has Deco-era network config. Needs manual check on Dell side.
- **CrowdSec plugin naming**: Debian-packaged plugins in `/usr/lib/crowdsec/plugins/` named `http` instead of `notification-http`. Required manual rename.
- **Cargo PATH on SHANNON**: Non-interactive SSH doesn't source `.cargo/env`. Use `source /root/.cargo/env && cargo build` for builds.
- **WireGuard key parsing**: Base64 keys end with `=` — can't use `split('=')` for config parsing. Use `find('=')` position + slice.
