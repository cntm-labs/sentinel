//! Indicatif progress + ✓/⚠/✗ helpers coloured via `owo-colors` (MIT).

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;

pub fn progress(total: u64, label: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("{spinner} {msg} [{bar:30}] {pos}/{len} ({percent}%)")
            .expect("static template parses")
            .progress_chars("█▒"),
    );
    pb.set_message(label.to_string());
    pb
}

pub fn ok(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

pub fn warn(msg: &str) {
    println!("{} {}", "⚠".yellow(), msg);
}

pub fn err(msg: &str) {
    println!("{} {}", "✗".red(), msg);
}
