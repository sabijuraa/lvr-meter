use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

/// Styles used across all progress bars
fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {msg}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}",
    )
    .unwrap()
    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
    .progress_chars("█▉▊▋▌▍▎▏  ")
}

/// Top-level progress tracker for the fetch pipeline
pub struct FetchProgress {
    multi:        MultiProgress,
    overall:      ProgressBar,
    pool_count:   usize,
}

impl FetchProgress {
    pub fn new(pool_count: usize) -> Self {
        let multi   = MultiProgress::new();
        let overall = multi.add(ProgressBar::new(pool_count as u64));

        overall.set_style(bar_style());
        overall.set_message("Fetching pool transactions...");
        overall.enable_steady_tick(std::time::Duration::from_millis(100));

        Self { multi, overall, pool_count }
    }

    /// Create a spinner for a single pool fetch
    pub fn pool_spinner(&self, pool_short: &str, pool_index: usize) -> PoolProgress {
        let spinner = self.multi.add(ProgressBar::new_spinner());
        spinner.set_style(spinner_style());
        spinner.set_message(format!(
            "Pool {}/{} ({}) — starting...",
            pool_index + 1,
            self.pool_count,
            pool_short
        ));
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        PoolProgress {
            spinner,
            pool_short: pool_short.to_string(),
            pool_index,
            pool_count: self.pool_count,
            tx_count:   0,
            page:       0,
        }
    }

    /// Advance the overall bar by one pool
    pub fn pool_complete(&self, pool_short: &str, tx_count: usize) {
        self.overall.inc(1);
        self.overall.set_message(format!(
            "{} done ({} txs)",
            pool_short, tx_count
        ));
    }

    /// Finish the overall progress bar
    pub fn finish(&self, total_txs: usize, pool_count: usize) {
        self.overall.finish_with_message(format!(
            "Fetched {} transactions across {} pools — cached to .lvr-cache/",
            total_txs, pool_count
        ));
    }
}

/// Per-pool progress spinner
pub struct PoolProgress {
    spinner:    ProgressBar,
    pool_short: String,
    pool_index: usize,
    pool_count: usize,
    tx_count:   usize,
    page:       usize,
}

impl PoolProgress {
    pub fn update_page(&mut self, page: usize, tx_count: usize) {
        self.page     = page;
        self.tx_count = tx_count;
        self.spinner.set_message(format!(
            "Pool {}/{} ({}) — page {}, {:>6} txs so far",
            self.pool_index + 1,
            self.pool_count,
            self.pool_short,
            page,
            tx_count,
        ));
    }

    pub fn finish_cache_hit(&self, tx_count: usize) {
        self.spinner.finish_with_message(format!(
            "Pool {}/{} ({}) — cache hit ({} txs)",
            self.pool_index + 1,
            self.pool_count,
            self.pool_short,
            tx_count,
        ));
    }

    pub fn finish_fetched(&self, tx_count: usize) {
        self.spinner.finish_with_message(format!(
            "Pool {}/{} ({}) ✓ {} txs",
            self.pool_index + 1,
            self.pool_count,
            self.pool_short,
            tx_count,
        ));
    }

    pub fn tx_count(&self) -> usize {
        self.tx_count
    }
}

/// Simple phase header — printed before each major phase starts
pub fn print_phase_header(phase: &str) {
    println!("\n{}", "─".repeat(50));
    println!("  {}", phase);
    println!("{}", "─".repeat(50));
}

/// Parsing progress — deterministic, so use a plain bar
pub fn parsing_bar(total: usize) -> ProgressBar {
    let bar = ProgressBar::new(total as u64);
    bar.set_style(bar_style());
    bar.set_message("Parsing transactions...");
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_progress_creates_without_panic() {
        let progress = FetchProgress::new(5);
        let spinner  = progress.pool_spinner("7xKXtg", 0);
        spinner.finish_cache_hit(100);
        progress.pool_complete("7xKXtg", 100);
        progress.finish(500, 5);
    }

    #[test]
    fn pool_progress_update_does_not_panic() {
        let progress = FetchProgress::new(3);
        let mut sp   = progress.pool_spinner("ABCxyz", 1);
        sp.update_page(1, 1000);
        sp.update_page(2, 2000);
        sp.finish_fetched(2000);
    }

    #[test]
    fn parsing_bar_creates_without_panic() {
        let bar = parsing_bar(100);
        bar.inc(50);
        bar.finish_with_message("done");
    }
}