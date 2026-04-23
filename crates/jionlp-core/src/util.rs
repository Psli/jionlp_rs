//! Port of `jionlp/util/*.py` helpers that aren't already adequately
//! covered by Rust stdlib.
//!
//! * Regex compositors (`bracket`, `bracket_absence`, `absence`, `start_end`)
//!   — small string wrappers used when building patterns.
//! * `TimeIt` context timer wrapper — a simple `Instant`-based helper.
//!
//! Python I/O helpers (`read_file_by_line`, `write_file_by_line`) are
//! omitted — Rust's `std::fs::read_to_string` / `std::fs::write` are
//! idiomatic and require no wrapper. Same for `set_logger` (use `tracing`
//! or `log`) and `HelpSearch` (CLI tool, not a library concern).

/// Wrap a regex pattern in a capturing group: `bracket("foo") == "(foo)"`.
pub fn bracket(pat: &str) -> String {
    format!("({})", pat)
}

/// Optional capturing group: `bracket_absence("foo") == "(foo)?"`.
pub fn bracket_absence(pat: &str) -> String {
    format!("({})?", pat)
}

/// Append `?` — make a pattern optional: `absence("foo") == "foo?"`.
pub fn absence(pat: &str) -> String {
    format!("{}?", pat)
}

/// Anchor both ends: `start_end("foo") == "^foo$"`.
pub fn start_end(pat: &str) -> String {
    format!("^{}$", pat)
}

/// Simple timer — mimics Python's `TimeIt` context manager.
pub struct TimeIt {
    start: std::time::Instant,
    label: String,
}

impl TimeIt {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            start: std::time::Instant::now(),
            label: label.into(),
        }
    }
    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl Drop for TimeIt {
    fn drop(&mut self) {
        let ms = self.elapsed_ms();
        eprintln!("[TimeIt:{}] {} ms", self.label, ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_composers() {
        assert_eq!(bracket("abc"), "(abc)");
        assert_eq!(bracket_absence("abc"), "(abc)?");
        assert_eq!(absence("abc"), "abc?");
        assert_eq!(start_end("abc"), "^abc$");
    }

    #[test]
    fn timeit_runs() {
        let t = TimeIt::new("unit-test");
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(t.elapsed_ms() >= 1);
        assert_eq!(t.label(), "unit-test");
    }
}
