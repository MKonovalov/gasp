use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut repo: Option<PathBuf> = None;
    let mut fixture_facts = false;
    for arg in &mut args {
        match arg.as_str() {
            "--fixture" => fixture_facts = true,
            "-h" | "--help" => {
                println!("usage: conformance-check <agent-repo> [--fixture]");
                println!("  --fixture   additionally assert the Part VI fixture graph facts");
                return ExitCode::SUCCESS;
            }
            path => repo = Some(PathBuf::from(path)),
        }
    }
    let Some(repo) = repo else {
        eprintln!("usage: conformance-check <agent-repo> [--fixture]");
        return ExitCode::from(2);
    };

    let reports = match conformance_check::run_all(&repo, fixture_facts) {
        Ok(reports) => reports,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let mut failed = false;
    for report in &reports {
        let mark = if report.passed() { "PASS" } else { "FAIL" };
        println!("[{mark}] check {} — {}", report.number, report.name);
        for note in &report.notes {
            println!("       note: {note}");
        }
        for failure in &report.failures {
            println!("       {failure}");
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        println!("conformant: all checks passed");
        ExitCode::SUCCESS
    }
}
