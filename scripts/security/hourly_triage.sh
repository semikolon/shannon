#!/bin/bash
# Hourly security triage via GPT-5-nano
# Categorizes: critical / normal / clear
# Critical → ntfy urgent topic
# Normal → append to daily digest
# Clear → log only

set -euo pipefail

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
LOG_FILE="/var/log/shannon-llm-triage.log"
DIGEST_FILE="/var/log/shannon-daily-digest.jsonl"
OPENAI_API_KEY="${OPENAI_API_KEY:-}"

if [ -z "$OPENAI_API_KEY" ]; then
  if [ -f /etc/shannon-security/env ]; then
    source /etc/shannon-security/env
  else
    echo "$(date -Iseconds) ERROR: No OPENAI_API_KEY set" >> "$LOG_FILE"
    exit 1
  fi
fi

# Collect last hour of logs
LOGS=$("$SCRIPT_DIR/collect_logs.sh" --since 1h 2>/dev/null)

if [ -z "$LOGS" ]; then
  echo "$(date -Iseconds) WARN: No logs collected" >> "$LOG_FILE"
  exit 0
fi

# Write prompt to temp file (avoids shell argument size limits)
PROMPT_FILE=$(mktemp /tmp/shannon-hourly-prompt.XXXXXX)
REQUEST_FILE=""
trap 'rm -f "$PROMPT_FILE" "$REQUEST_FILE"' EXIT

cat > "$PROMPT_FILE" <<PROMPTEOF
You are a security analyst for a home network router (Rock Pi 4B, Armbian Linux).
Analyze these logs from the last hour and categorize the overall security status.

Respond with ONLY valid JSON, no markdown formatting, no code fences:
{"category": "critical|normal|clear", "summary": "one-line summary", "findings": ["finding1", "finding2"]}

- critical: Active attack, successful breach, or unusual pattern needing immediate attention
- normal: Expected activity (routine SSH bans, normal DNS traffic) worth noting in daily digest
- clear: Nothing noteworthy

Logs:
$LOGS
PROMPTEOF

# Build request JSON to file (avoids shell argument limits)
REQUEST_FILE=$(mktemp /tmp/shannon-hourly-request.XXXXXX)
jq -n --rawfile prompt "$PROMPT_FILE" '{
  model: "gpt-5-nano",
  messages: [{"role": "user", "content": $prompt}],
  max_completion_tokens: 2000
}' > "$REQUEST_FILE"

# Call GPT-5-nano (use @file to avoid argument size limits)
RESPONSE=$(curl -s --max-time 60 https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d @"$REQUEST_FILE" 2>/dev/null)

if [ -z "$RESPONSE" ]; then
  echo "$(date -Iseconds) ERROR: Empty API response" >> "$LOG_FILE"
  exit 1
fi

# Extract content from API response
RAW_CONTENT=$(echo "$RESPONSE" | jq -r '.choices[0].message.content // empty' 2>/dev/null)

if [ -z "$RAW_CONTENT" ]; then
  ERROR=$(echo "$RESPONSE" | jq -r '.error.message // "unknown error"' 2>/dev/null)
  echo "$(date -Iseconds) ERROR: API error: $ERROR" >> "$LOG_FILE"
  exit 1
fi

# Parse JSON from LLM response (handles code fences, multiline)
RAW_FILE=$(mktemp /tmp/shannon-hourly-raw.XXXXXX)
echo "$RAW_CONTENT" | sed '/^```json$/d; /^```$/d' > "$RAW_FILE"
CONTENT=$(jq -c '.' "$RAW_FILE" 2>/dev/null || true)
rm -f "$RAW_FILE"

if [ -z "$CONTENT" ]; then
  echo "$(date -Iseconds) WARN: Could not parse LLM response as JSON, logging raw" >> "$LOG_FILE"
  echo "$(date -Iseconds) RAW: $RAW_CONTENT" >> "$LOG_FILE"
  exit 0
fi

# Parse category
CATEGORY=$(echo "$CONTENT" | jq -r '.category // "clear"' 2>/dev/null || echo "clear")
SUMMARY=$(echo "$CONTENT" | jq -r '.summary // "No summary"' 2>/dev/null || echo "Parse error")

# Log the run
echo "$(date -Iseconds) TRIAGE: category=$CATEGORY summary=\"$SUMMARY\"" >> "$LOG_FILE"

case "$CATEGORY" in
  critical)
    # Send to ntfy urgent topic
    curl -s --max-time 5 -d "SHANNON SECURITY ALERT: $SUMMARY" \
      http://192.168.4.84:8099/shannon-security-urgent 2>/dev/null || true
    # Also append to digest
    echo "{\"timestamp\":\"$(date -Iseconds)\",\"category\":\"critical\",\"content\":$CONTENT}" >> "$DIGEST_FILE"
    ;;
  normal)
    # Append to daily digest for Gemini analysis
    echo "{\"timestamp\":\"$(date -Iseconds)\",\"category\":\"normal\",\"content\":$CONTENT}" >> "$DIGEST_FILE"
    ;;
  clear)
    # Log only, nothing to action
    ;;
esac

echo "$(date -Iseconds) OK: Hourly triage complete" >> "$LOG_FILE"
