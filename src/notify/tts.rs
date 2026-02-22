//! TTS notification via daemon socket (placeholder)

// TODO: Implement TTS notification by writing to daemon socket
// See ~/.claude/hooks/tts_daemon.py for protocol

pub fn notify_tts(_message: &str) -> anyhow::Result<()> {
    // TODO: Write to TTS daemon socket/file
    Ok(())
}
