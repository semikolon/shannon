//! Output formatting for plain text and JSON

use serde::Serialize;
use std::fmt::Display;

/// Format output based on --json flag
pub fn format_output<T: Serialize + Display>(data: &T, json: bool) -> String {
    if json {
        serde_json::to_string_pretty(data).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    } else {
        data.to_string()
    }
}

/// Print output with newline
pub fn print_output<T: Serialize + Display>(data: &T, json: bool) {
    println!("{}", format_output(data, json));
}

/// Simple key-value output for status displays
#[derive(Debug, Serialize)]
pub struct StatusLine {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<LineStatus>,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LineStatus {
    Ok,
    Warning,
    Error,
}

impl Display for StatusLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indicator = match self.status {
            Some(LineStatus::Ok) => "✓",
            Some(LineStatus::Warning) => "!",
            Some(LineStatus::Error) => "✗",
            None => " ",
        };
        write!(f, "{} {}: {}", indicator, self.key, self.value)
    }
}

/// Collection of status lines for multi-line output
#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub lines: Vec<StatusLine>,
}

impl Display for StatusReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in &self.lines {
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }
}

/// Table output for lists (DNS records, DHCP leases, etc.)
#[derive(Debug, Serialize)]
pub struct TableOutput<T: Serialize> {
    pub headers: Vec<String>,
    pub rows: Vec<T>,
}

impl<T: Serialize + TableRow> Display for TableOutput<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Calculate column widths
        let mut widths: Vec<usize> = self.headers.iter().map(|h| h.len()).collect();
        for row in &self.rows {
            let cells = row.cells();
            for (i, cell) in cells.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        // Print header
        for (i, header) in self.headers.iter().enumerate() {
            if i > 0 {
                write!(f, "  ")?;
            }
            write!(f, "{:width$}", header, width = widths[i])?;
        }
        writeln!(f)?;

        // Print separator
        for (i, width) in widths.iter().enumerate() {
            if i > 0 {
                write!(f, "  ")?;
            }
            write!(f, "{}", "-".repeat(*width))?;
        }
        writeln!(f)?;

        // Print rows
        for row in &self.rows {
            let cells = row.cells();
            for (i, cell) in cells.iter().enumerate() {
                if i > 0 {
                    write!(f, "  ")?;
                }
                if i < widths.len() {
                    write!(f, "{:width$}", cell, width = widths[i])?;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

/// Trait for table row formatting
pub trait TableRow {
    fn cells(&self) -> Vec<String>;
}

/// Confirmation prompt
pub fn confirm(message: &str, yes_flag: bool) -> bool {
    if yes_flag {
        return true;
    }

    eprint!("{} [y/N] ", message);

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}
