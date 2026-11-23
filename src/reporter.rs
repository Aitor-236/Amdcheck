use std::io::{self, Write};

use crate::checker::{CheckResult, CheckStatus};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReportSummary {
    pub ok: usize,
    pub broken: usize,
    pub skipped: usize,
}

pub fn summarize(results: &[CheckResult]) -> ReportSummary {
    let mut summary = ReportSummary::default();

    for result in results {
        match &result.status {
            CheckStatus::Ok => summary.ok += 1,
            CheckStatus::Broken { .. } => summary.broken += 1,
            CheckStatus::Skipped { .. } => summary.skipped += 1,
        }
    }

    summary
}

pub fn print_report<W: Write>(writer: &mut W, results: &[CheckResult]) -> io::Result<()> {
    for result in results {
        if let CheckStatus::Broken { reason } = &result.status {
            writeln!(
                writer,
                "BROKEN {}:{} - {} ({})",
                result.file.display(),
                result.line,
                result.destination,
                reason
            )?;
        }
    }

    let summary = summarize(results);
    writeln!(
        writer,
        "\n{} valid, {} broken, {} skipped",
        summary.ok, summary.broken, summary.skipped
    )
}

pub fn has_broken_links(results: &[CheckResult]) -> bool {
    results
        .iter()
        .any(|result| matches!(result.status, CheckStatus::Broken { .. }))
}
