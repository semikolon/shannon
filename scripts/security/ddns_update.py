#!/usr/bin/env python3
"""SHANNON Dynamic DNS updater via Loopia XMLRPC API.

Updates the 'shannon' A record at fredrikbranstrom.se to match
the current WAN IP. Only makes API calls when the IP changes.

Designed to run via systemd timer every 5 minutes.

Usage:
  shannon-ddns          # Check and update if needed
  shannon-ddns --force  # Force update even if IP unchanged
  shannon-ddns --status # Show current DDNS state
"""

import subprocess
import sys
import json
import xmlrpc.client
from datetime import datetime
from pathlib import Path

# Configuration
DOMAIN = "fredrikbranstrom.se"
SUBDOMAIN = "shannon"
WAN_INTERFACE = "enxc84d4421f975"
CACHE_FILE = "/var/cache/shannon-ddns-ip"
STATE_FILE = "/var/cache/shannon-ddns-state.json"
LOG_FILE = "/var/log/shannon-llm-triage.log"
ENV_FILE = "/etc/shannon-security/env"
NTFY_URL = "http://192.168.4.84:8099/shannon-security"
TTL = 300  # 5 minutes
LOOPIA_API = "https://api.loopia.se/RPCSERV"


def log(level, msg):
    """Log to shared shannon log file and stderr."""
    ts = datetime.now().isoformat(timespec="seconds")
    line = f"{ts} DDNS: {level}: {msg}"
    print(line, file=sys.stderr)
    try:
        with open(LOG_FILE, "a") as f:
            f.write(line + "\n")
    except Exception:
        pass


def get_wan_ip():
    """Read WAN IP directly from the interface (no external HTTP call needed)."""
    try:
        result = subprocess.run(
            ["ip", "-4", "-o", "addr", "show", WAN_INTERFACE],
            capture_output=True, text=True, timeout=5
        )
        # Output format: "5: enxc84d4421f975    inet 94.254.88.116/24 ..."
        for part in result.stdout.split():
            if "/" in part and "." in part:
                ip = part.split("/")[0]
                # Validate it looks like an IP
                octets = ip.split(".")
                if len(octets) == 4 and all(o.isdigit() and 0 <= int(o) <= 255 for o in octets):
                    return ip
    except Exception as e:
        log("ERROR", f"Failed to read WAN IP from {WAN_INTERFACE}: {e}")
    return None


def get_cached_ip():
    """Read last known IP from cache."""
    try:
        return Path(CACHE_FILE).read_text().strip()
    except FileNotFoundError:
        return None


def save_cached_ip(ip):
    """Save current IP to cache."""
    Path(CACHE_FILE).write_text(ip + "\n")


def save_state(ip, status, message=""):
    """Save full state for CLI status command."""
    state = {
        "ip": ip,
        "status": status,
        "message": message,
        "timestamp": datetime.now().isoformat(timespec="seconds"),
        "fqdn": f"{SUBDOMAIN}.{DOMAIN}",
        "interface": WAN_INTERFACE,
        "ttl": TTL,
    }
    try:
        Path(STATE_FILE).write_text(json.dumps(state, indent=2) + "\n")
    except Exception:
        pass


def load_loopia_creds():
    """Load Loopia API credentials from env file."""
    creds = {}
    try:
        with open(ENV_FILE) as f:
            for line in f:
                line = line.strip()
                if line.startswith("#") or "=" not in line:
                    continue
                key, _, val = line.partition("=")
                creds[key.strip()] = val.strip().strip("'\"")
    except FileNotFoundError:
        log("ERROR", f"Credentials file not found: {ENV_FILE}")
        sys.exit(1)

    user = creds.get("LOOPIA_USER")
    password = creds.get("LOOPIA_PASSWORD")
    if not user or not password:
        log("ERROR", "LOOPIA_USER and LOOPIA_PASSWORD required in " + ENV_FILE)
        sys.exit(1)
    return user, password


def update_dns(user, password, ip):
    """Update or create the A record via Loopia XMLRPC API."""
    client = xmlrpc.client.ServerProxy(LOOPIA_API, encoding="utf-8")

    # Get existing records for the subdomain
    try:
        records = client.getZoneRecords(user, password, DOMAIN, SUBDOMAIN)
    except xmlrpc.client.Fault as e:
        log("ERROR", f"Loopia API fault: code={e.faultCode} msg={e.faultString}")
        return False
    except Exception as e:
        log("ERROR", f"Loopia API connection failed: {e}")
        return False

    # Handle API error responses (Loopia returns status strings on error)
    if isinstance(records, str):
        if records == "AUTH_ERROR":
            log("ERROR", "Loopia API authentication failed — check LOOPIA_USER/LOOPIA_PASSWORD")
        elif records == "UNKNOWN_ERROR":
            log("ERROR", "Loopia API returned UNKNOWN_ERROR — subdomain may not exist yet")
        else:
            log("ERROR", f"Loopia API error: {records}")
        return False

    # Find existing A record
    a_record = None
    if isinstance(records, list):
        for r in records:
            if isinstance(r, dict) and r.get("type") == "A":
                a_record = r
                break

    if a_record:
        # Check if already correct
        if a_record.get("rdata") == ip and a_record.get("ttl") == TTL:
            log("INFO", f"DNS already correct ({ip}), skipping API call")
            return True

        # Update existing record
        a_record["rdata"] = ip
        a_record["ttl"] = TTL
        try:
            result = client.updateZoneRecord(user, password, DOMAIN, SUBDOMAIN, a_record)
            if result == "OK":
                log("OK", f"Updated existing A record → {ip}")
                return True
            else:
                log("ERROR", f"updateZoneRecord returned: {result}")
                return False
        except Exception as e:
            log("ERROR", f"updateZoneRecord failed: {e}")
            return False
    else:
        # No A record exists — create subdomain + record
        try:
            sub_result = client.addSubdomain(user, password, DOMAIN, SUBDOMAIN)
            if sub_result not in ("OK", "DOMAIN_OCCUPIED"):
                log("WARN", f"addSubdomain returned: {sub_result}")
        except Exception:
            pass  # Subdomain may already exist

        new_record = {
            "type": "A",
            "ttl": TTL,
            "priority": 0,
            "rdata": ip,
        }
        try:
            result = client.addZoneRecord(user, password, DOMAIN, SUBDOMAIN, new_record)
            if result == "OK":
                log("OK", f"Created new A record → {ip}")
                return True
            else:
                log("ERROR", f"addZoneRecord returned: {result}")
                return False
        except Exception as e:
            log("ERROR", f"addZoneRecord failed: {e}")
            return False


def notify(message):
    """Send notification via ntfy (best-effort, non-blocking)."""
    try:
        subprocess.run(
            ["curl", "-s", "--max-time", "5", "-d", message, NTFY_URL],
            capture_output=True, timeout=10
        )
    except Exception:
        pass


def show_status():
    """Display current DDNS state."""
    wan_ip = get_wan_ip() or "unknown"
    cached_ip = get_cached_ip() or "never updated"

    try:
        state = json.loads(Path(STATE_FILE).read_text())
    except Exception:
        state = {}

    print(f"DDNS: {SUBDOMAIN}.{DOMAIN}")
    print(f"  WAN IP:     {wan_ip}")
    print(f"  DNS record: {cached_ip}")
    print(f"  Status:     {state.get('status', 'unknown')}")
    print(f"  Last check: {state.get('timestamp', 'never')}")
    print(f"  Message:    {state.get('message', 'n/a')}")
    print(f"  Interface:  {WAN_INTERFACE}")
    print(f"  TTL:        {TTL}s")

    if wan_ip != "unknown" and cached_ip != "never updated" and wan_ip != cached_ip:
        print(f"  ⚠ IP MISMATCH — DNS needs update")


def main():
    # Handle --status flag
    if "--status" in sys.argv:
        show_status()
        return

    force = "--force" in sys.argv

    current_ip = get_wan_ip()
    if not current_ip:
        log("ERROR", "Could not determine WAN IP")
        save_state("unknown", "error", "Could not read WAN interface")
        sys.exit(1)

    cached_ip = get_cached_ip()

    if cached_ip == current_ip and not force:
        # No change — exit silently (no log noise)
        save_state(current_ip, "ok", "No change")
        return

    # IP changed (or first run, or --force)
    old_ip = cached_ip or "(first run)"
    if force and cached_ip == current_ip:
        log("INFO", f"Force update: {current_ip}")
    else:
        log("INFO", f"IP change detected: {old_ip} → {current_ip}")

    user, password = load_loopia_creds()

    if update_dns(user, password, current_ip):
        save_cached_ip(current_ip)
        save_state(current_ip, "updated", f"Changed from {old_ip}")
        log("OK", f"Updated {SUBDOMAIN}.{DOMAIN} → {current_ip}")

        # Notify on change (not on first run to avoid spam)
        if cached_ip and cached_ip != current_ip:
            notify(f"SHANNON IP changed: {cached_ip} → {current_ip} ({SUBDOMAIN}.{DOMAIN})")
    else:
        save_state(current_ip, "error", "DNS update failed")
        log("ERROR", f"Failed to update DNS to {current_ip}")
        sys.exit(1)


if __name__ == "__main__":
    main()
