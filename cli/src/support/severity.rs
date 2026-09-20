//! Colouring a [`Severity`] for the terminal.

use crate::commands::patch::Severity;
use colored::Colorize;

pub(crate) fn severity_colored(sev: &Severity) -> colored::ColoredString {
    match sev {
        Severity::Critical => "CRITICAL".red().bold(),
        Severity::High => "HIGH".yellow().bold(),
        Severity::Medium => "MEDIUM".cyan(),
        Severity::Low => "LOW".normal(),
    }
}
