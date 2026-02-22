#!/bin/bash
# Daily deep security analysis via Gemini 3 Pro
# Correlates 24h of logs + hourly triage findings
# Produces: pattern analysis, behavioral anomalies, trend detection

set -euo pipefail

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
LOG_FILE="/var/log/shannon-llm-triage.log"
DIGEST_FILE="/var/log/shannon-daily-digest.jsonl"
GEMINI_API_KEY="${GEMINI_API_KEY:-}"
GEMINI_MODEL="${GEMINI_MODEL:-gemini-2.5-flash}"

if [ -z "$GEMINI_API_KEY" ]; then
  if [ -f /etc/shannon-security/env ]; then
    source /etc/shannon-security/env
  else
    echo "$(date -Iseconds) ERROR: No GEMINI_API_KEY set" >> "$LOG_FILE"
    exit 1
  fi
fi

# Collect last 24h of logs
LOGS=$("$SCRIPT_DIR/collect_logs.sh" --since 24h 2>/dev/null)

# Read hourly triage digest (last 24h entries)
HOURLY_DIGEST=""
if [ -f "$DIGEST_FILE" ]; then
  HOURLY_DIGEST=$(cat "$DIGEST_FILE" 2>/dev/null || echo "")
fi

# Write prompt to temp file (avoids shell argument size limits)
PROMPT_FILE=$(mktemp /tmp/shannon-daily-prompt.XXXXXX)
REQUEST_FILE=""
trap 'rm -f "$PROMPT_FILE" "$REQUEST_FILE"' EXIT

cat > "$PROMPT_FILE" <<PROMPTEOF
You are a senior security analyst performing daily review of a home network router (Rock Pi 4B SE, Armbian Linux, acting as upstream router).

Security stack: CrowdSec (IDS with nftables bouncer), AdGuard Home (DNS filtering), WireGuard (VPN).

Analyze the last 24 hours of data and produce a security digest.

## Raw System Logs (24h)
$LOGS

## Hourly Triage Findings (GPT-5-nano summaries)
$HOURLY_DIGEST

## Analysis Required
1. **Pattern Correlation**: Are there related events across different log sources?
2. **Behavioral Anomalies**: Anything unusual compared to expected home network behavior?
3. **Trend Detection**: Increasing attack frequency? New attack vectors? DNS anomalies?
4. **CrowdSec Effectiveness**: Are the right IPs being blocked? Any gaps?
5. **Recommendations**: Actionable security improvements?

Respond with ONLY valid JSON, no markdown formatting, no code fences:
{
  "severity": "green|yellow|red",
  "summary": "2-3 sentence overview",
  "patterns": ["pattern descriptions"],
  "anomalies": ["anomaly descriptions"],
  "trends": ["trend descriptions"],
  "recommendations": ["actionable items"],
  "escalation_needed": false,
  "escalation_prompt": null
}

If escalation_needed is true, include an escalation_prompt that can be pasted into Claude Code for investigation.
PROMPTEOF

# Build request JSON to file (avoids shell argument limits for curl too)
REQUEST_FILE=$(mktemp /tmp/shannon-daily-request.XXXXXX)

jq -n --rawfile prompt "$PROMPT_FILE" '{
  contents: [{"parts": [{"text": $prompt}]}],
  generationConfig: {
    temperature: 0.2,
    maxOutputTokens: 8192
  }
}' > "$REQUEST_FILE"

# Call Gemini 3 Pro (use @file to avoid argument size limits)
RESPONSE=$(curl -s --max-time 120 \
  "https://generativelanguage.googleapis.com/v1beta/models/$GEMINI_MODEL:generateContent?key=$GEMINI_API_KEY" \
  -H "Content-Type: application/json" \
  -d @"$REQUEST_FILE" 2>/dev/null)

if [ -z "$RESPONSE" ]; then
  echo "$(date -Iseconds) ERROR: Empty Gemini API response" >> "$LOG_FILE"
  exit 1
fi

# Extract content
RAW_CONTENT=$(echo "$RESPONSE" | jq -r '.candidates[0].content.parts[0].text // empty' 2>/dev/null)

if [ -z "$RAW_CONTENT" ]; then
  ERROR=$(echo "$RESPONSE" | jq -r '.error.message // "unknown error"' 2>/dev/null)
  echo "$(date -Iseconds) ERROR: Gemini API error: $ERROR" >> "$LOG_FILE"
  exit 1
fi

# Parse JSON from LLM response (handles code fences, multiline)
# Write to temp file for reliable multiline jq parsing (printf preserves content)
RAW_FILE=$(mktemp /tmp/shannon-daily-raw.XXXXXX)
printf '%s\n' "$RAW_CONTENT" | sed '/^```json$/d; /^```$/d' > "$RAW_FILE"
CONTENT=$(jq -c '.' "$RAW_FILE" 2>/dev/null || true)
rm -f "$RAW_FILE"

if [ -z "$CONTENT" ]; then
  echo "$(date -Iseconds) WARN: Could not parse Gemini response as JSON, logging raw" >> "$LOG_FILE"
  echo "$(date -Iseconds) RAW: $RAW_CONTENT" >> "$LOG_FILE"
  exit 0
fi

# Parse severity
SEVERITY=$(echo "$CONTENT" | jq -r '.severity // "green"' 2>/dev/null || echo "green")
SUMMARY=$(echo "$CONTENT" | jq -r '.summary // "No summary"' 2>/dev/null || echo "Parse error")
ESCALATION=$(echo "$CONTENT" | jq -r '.escalation_needed // false' 2>/dev/null || echo "false")

# Log the run
echo "$(date -Iseconds) DAILY: severity=$SEVERITY summary=\"$SUMMARY\"" >> "$LOG_FILE"

# Send digest to ntfy (when Dell is reachable)
DIGEST_MSG="SHANNON Daily Security Digest [$SEVERITY]: $SUMMARY"
curl -s --max-time 5 -d "$DIGEST_MSG" \
  http://192.168.4.84:8099/shannon-security 2>/dev/null || true

# Save full analysis
ANALYSIS_DIR="/var/log/shannon-security-analyses"
mkdir -p "$ANALYSIS_DIR"
echo "$CONTENT" > "$ANALYSIS_DIR/$(date +%Y-%m-%d).json"

# If escalation needed, save prompt
if [ "$ESCALATION" = "true" ]; then
  ESCALATION_PROMPT=$(echo "$CONTENT" | jq -r '.escalation_prompt // empty' 2>/dev/null)
  if [ -n "$ESCALATION_PROMPT" ]; then
    echo "$ESCALATION_PROMPT" > "$ANALYSIS_DIR/$(date +%Y-%m-%d)_escalation.txt"
    # Send urgent notification
    curl -s --max-time 5 -d "SHANNON ESCALATION: $SUMMARY - check /var/log/shannon-security-analyses/" \
      http://192.168.4.84:8099/shannon-security-urgent 2>/dev/null || true
  fi
fi

# Rotate daily digest (clear for next day)
> "$DIGEST_FILE"

echo "$(date -Iseconds) OK: Daily analysis complete" >> "$LOG_FILE"
