#!/bin/bash
# Collect security-relevant logs for LLM analysis
# Usage: collect_logs.sh --since 1h   (for hourly triage)
#        collect_logs.sh --since 24h  (for daily analysis)

set -euo pipefail

SINCE="${2:-1h}"

# Map to journalctl format
case "$SINCE" in
  1h)  JOURNAL_SINCE="1 hour ago" ;;
  24h) JOURNAL_SINCE="24 hours ago" ;;
  *)   JOURNAL_SINCE="$SINCE" ;;
esac

# Collect auth.log entries
AUTH_LOGS=$(journalctl -u ssh --since "$JOURNAL_SINCE" --no-pager -o short-iso 2>/dev/null | tail -500 || echo "")

# Collect syslog entries (security-relevant)
SYSLOG=$(journalctl --since "$JOURNAL_SINCE" --no-pager -o short-iso -p warning 2>/dev/null | tail -200 || echo "")

# CrowdSec decisions
CROWDSEC_DECISIONS=$(cscli decisions list -o json 2>/dev/null || echo "[]")

# CrowdSec alerts (recent)
CROWDSEC_ALERTS=$(cscli alerts list -o json --since "$SINCE" 2>/dev/null || echo "[]")

# Network stats
CONNECTIONS=$(ss -tun state established 2>/dev/null | wc -l || echo "0")
LISTENING=$(ss -tlun 2>/dev/null | tail -n +2 || echo "")

# AdGuard stats
ADGUARD_STATS=$(curl -s -u admin:shannon-admin-2026 http://localhost:3000/control/stats 2>/dev/null || echo "{}")

# --- System state context (gives LLM awareness of service health + self-healing) ---

# Key service states with uptime
SERVICES="crowdsec AdGuardHome wg-quick@wg0 dnsmasq ssh shannon-ddns.timer"
SERVICE_STATUS=""
for svc in $SERVICES; do
  state=$(systemctl is-active "$svc" 2>/dev/null || echo "unknown")
  since=$(systemctl show "$svc" --property=ActiveEnterTimestamp --value 2>/dev/null || echo "")
  SERVICE_STATUS="${SERVICE_STATUS}{\"service\":\"$svc\",\"state\":\"$state\",\"active_since\":\"$since\"},"
done
SERVICE_STATUS="[${SERVICE_STATUS%,}]"

# Service lifecycle events in period (Started/Stopped/Failed — reveals crashes + auto-recoveries)
SERVICE_EVENTS=$(journalctl --since "$JOURNAL_SINCE" --no-pager -o short-iso \
  -u crowdsec -u AdGuardHome -u "wg-quick@wg0" -u dnsmasq -u ssh -u shannon-ddns \
  --grep="Started|Stopped|Failed|Deactivated|Main process exited" 2>/dev/null | tail -50 || echo "")

# System resources
UPTIME=$(uptime 2>/dev/null || echo "")
MEMORY=$(free -m 2>/dev/null | awk '/^Mem:/{printf "{\"total_mb\":%d,\"used_mb\":%d,\"available_mb\":%d,\"percent_used\":%.1f}", $2, $3, $7, ($3*100)/$2}' || echo "{}")
DISK=$(df -h / 2>/dev/null | awk 'NR==2{printf "{\"total\":\"%s\",\"used\":\"%s\",\"available\":\"%s\",\"percent_used\":\"%s\"}", $2, $3, $4, $5}' || echo "{}")
LOAD=$(cat /proc/loadavg 2>/dev/null | awk '{printf "{\"1min\":%s,\"5min\":%s,\"15min\":%s}", $1, $2, $3}' || echo "{}")
TEMP=$(cat /sys/class/thermal/thermal_zone0/temp 2>/dev/null | awk '{printf "%.1f", $1/1000}' || echo "0")

# Output as JSON
cat <<EOF
{
  "timestamp": "$(date -Iseconds)",
  "period": "$SINCE",
  "hostname": "shannon",
  "system": {
    "uptime": $(echo "$UPTIME" | jq -Rs .),
    "memory": $MEMORY,
    "disk": $DISK,
    "load": $LOAD,
    "cpu_temp_c": $TEMP
  },
  "services": {
    "current_state": $SERVICE_STATUS,
    "events_in_period": $(echo "$SERVICE_EVENTS" | jq -Rs .)
  },
  "auth_logs": $(echo "$AUTH_LOGS" | jq -Rs .),
  "syslog_warnings": $(echo "$SYSLOG" | jq -Rs .),
  "crowdsec": {
    "decisions": $CROWDSEC_DECISIONS,
    "alerts": $CROWDSEC_ALERTS
  },
  "network": {
    "established_connections": $CONNECTIONS,
    "listening_ports": $(echo "$LISTENING" | jq -Rs .)
  },
  "adguard_stats": $ADGUARD_STATS
}
EOF
