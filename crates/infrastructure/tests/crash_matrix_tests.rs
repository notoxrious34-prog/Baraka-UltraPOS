//! Crash & power-failure matrix (TASK-BRK-055, ADR-063).
//!
//! Deterministic kill-point testing of the atomic sale pipeline
//! (Architecture Baseline §12 crash / power-failure model, §31 power-cut
//! strategy, §5.3 retry policy, §13 outbox guarantees).
//!
//! ## Architecture
//!
//! Single binary, re-exec pattern: the parent runner (default mode)
//! spawns `current_exe` as `--crash-child`, `--retry-child`, or
//! `--concurrent-child` processes over temporary file-backed databases.
//! The child driver builds the same application/infrastructure stack the
//! integration tests use, registers SQLite's own connection hooks, runs
//! one checkout, and dies or reports. The parent owns temp-dir
//! lifecycle, child timeouts, and all post-crash verification.
//!
//! ## ADR-064 (in-transaction idempotency claim — canonical)
//!
//! The duplicate-claim SELECT is the FIRST in-transaction step and the
//! command_log INSERT (with response payload) is the FINAL
//! in-transaction step, atomically with the sale. Claim-without-result
//! is structurally impossible; every retry after any crash completes
//! cleanly. The command_log row is therefore an EXACT commit witness
//! (V5), not a heuristic.
//!
//! ## Expected outcomes (Directive #1, Part 3)
//!
//! KP-1..KP-6 and KP-7a → NOT COMMITTED (V7 + V4 + retry "committed-new").
//! KP-7b → COMMITTED (V6 + retry "cached"). KP-7a striking post-commit
//! is a HALT-and-report event, never silently accepted.
//!
//! ## Honesty boundary
//!
//! This matrix simulates SUDDEN PROCESS DEATH (`std::process::abort` —
//! failfast, no unwinding, no atexit). OS-level power loss is NOT
//! simulated; it is covered by SQLite's durability contract (WAL +
//! synchronous=FULL: fsync before COMMIT returns). KP-7a (pre-commit
//! instant) and KP-7b (post-return) bracket the response-delivery
//! window from both sides; the interior instant (commit frame written,
//! COMMIT not yet returned) is unreachable with the pinned rusqlite
//! hooks and is covered by SQLite's atomic commit guarantee, which
//! V1/V5 verify on every recovery.
//!
//! ## Out of scope
//!
//! Kit expansion, mixed payment, returns, and non-sale flows have their
//! own suites. CP-8 (printer-failure independence) is covered by
//! citation (see below); CP-9 is the bounded concurrency smoke in this
//! file. The Tauri single-instance lock is a startup concern: drivers
//! exercise the SQLite layer directly, which is exactly the
//! multi-process access class that recovery windows and backup/restore
//! create.
//!
//! ## CP-8 citations (print_worker.rs — no additions made)
//!
//! (a) committed sale survives print failure:
//!     `failed_print_leaves_accounting_byte_identical` (commits via the
//!     sale use-case, then forces a render failure; sale rows persist).
//! (b) accounting independent of printer state: same test asserts the
//!     full accounting snapshot is byte-identical after the failure.
//! (c) FAILED terminal state: `permanent_failure_marks_failed_after_three_sends`
//!     asserts status "FAILED" with bounded attempts; PENDING-retryable
//!     is covered by `transient_failures_retry_with_backoff_then_done`
//!     and `stale_processing_reset_on_startup_cycle`.

#[path = "crash_matrix/driver.rs"]
mod driver;
#[path = "crash_matrix/scenario.rs"]
mod scenario;
#[path = "crash_matrix/verify.rs"]
mod verify;

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use scenario::{
    command_id_for, db_path, flag_value, parse_status_line, KillPoint, Scenario, SCENARIOS,
};

const CHILD_TIMEOUT_SECS: u64 = 30;
const POLLER_CAP_SECS: u64 = 10;

/// Wait for a child with a hard bound. `None` = timeout (caller kills).
fn wait_bounded(child: &mut Child, secs: u64) -> Option<std::process::ExitStatus> {
    let start = Instant::now();
    let limit = Duration::from_secs(secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if start.elapsed() >= limit {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

/// Kill a timed-out child and wait for the kill to land (D3: the temp
/// dir is dropped only after the process is confirmed gone, so Windows
/// never sees a live handle on a removed directory).
fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let start = Instant::now();
    while child
        .try_wait()
        .map(|status| status.is_none())
        .unwrap_or(false)
        && start.elapsed() < Duration::from_secs(5)
    {
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn spawn_child(mode: &str, target: &str, db: &Path) -> Child {
    let exe = env::current_exe().expect("current exe");
    Command::new(exe)
        .arg(mode)
        .arg(target)
        .arg("--db")
        .arg(db)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn child")
}

fn child_output(child: &mut Child) -> (String, String) {
    use std::io::Read as _;
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    (stdout, stderr)
}

/// Run one kill scenario through kill → verify → retry → re-verify.
/// Returns `Ok(retry_status)` on pass, `Err(diagnostic)` on failure.
fn run_scenario(scenario: &Scenario) -> Result<String, String> {
    let dir = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let db = db_path(dir.path());
    let command_id = command_id_for(scenario.id);

    // 1. Kill child: must die abnormally (never exit 0, never Continue).
    let mut child = spawn_child("--crash-child", scenario.id, &db);
    let status = match wait_bounded(&mut child, CHILD_TIMEOUT_SECS) {
        Some(status) => status,
        None => {
            kill_and_reap(&mut child);
            return Err("kill child timed out after 30s".to_string());
        }
    };
    if status.success() {
        return Err(format!(
            "kill child exited normally ({status}) — hook never fired"
        ));
    }

    // 2. Post-crash battery on a plain reopen (SQLite auto-recovery only).
    let conn = verify::reopen(&db);
    verify::v1_integrity(&conn)?;
    verify::v2_audit_chain(&conn)?;
    verify::v3_reconciliation(&conn)?;
    let snap = verify::snapshot(&conn, &command_id);
    verify::v4_gapless(&conn)?;
    let committed = verify::v5_committed(snap.command_rows, snap.documents)?;

    // 3. Expected branch (Part 3 table). KP-7a observing COMMITTED is a
    // HALT event, never a silent flip.
    let expect_committed = scenario.point == KillPoint::Kp7bPostResponse;
    if committed != expect_committed {
        if scenario.point == KillPoint::Kp7aCommit && committed {
            return Err("HALT: KP-7a observed COMMITTED — contradicts documented \
                 pre-commit hook timing; requires architectural review"
                .to_string());
        }
        return Err(format!(
            "branch mismatch: expected committed={expect_committed}, observed {committed}"
        ));
    }
    if committed {
        verify::v6_committed_effects(&conn, scenario.kind, &snap)?;
    } else {
        verify::v7_no_artifacts(&conn, &snap)?;
    }
    let pre_number: String = if committed {
        conn.query_row("SELECT number FROM documents LIMIT 1", [], |row| row.get(0))
            .map_err(|e| format!("document number: {e}"))?
    } else {
        String::new()
    };
    let pre_payload: Option<String> = conn
        .query_row(
            "SELECT response_payload FROM command_log WHERE command_id = ?1",
            [&command_id],
            |row| row.get(0),
        )
        .ok();
    drop(conn);

    // 4. Retry child: exactly-once proof with the SAME command id.
    let mut retry = spawn_child("--retry-child", scenario.id, &db);
    let retry_status = match wait_bounded(&mut retry, CHILD_TIMEOUT_SECS) {
        Some(status) => status,
        None => {
            kill_and_reap(&mut retry);
            return Err("retry child timed out after 30s".to_string());
        }
    };
    let output = child_output(&mut retry);
    if !retry_status.success() {
        return Err(format!(
            "retry child failed ({retry_status}): out={} err={}",
            output.0, output.1
        ));
    }
    let (outcome, number) = output
        .0
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .find_map(parse_status_line)
        .ok_or_else(|| format!("retry child printed no status line: {output:?}"))?;
    let expect_outcome = if committed { "cached" } else { "committed-new" };
    if outcome != expect_outcome {
        return Err(format!(
            "retry outcome {outcome:?} != expected {expect_outcome:?}"
        ));
    }

    // 5. Post-retry: zero second effect OR exactly one effect set + V9.
    let conn = verify::reopen(&db);
    let after = verify::snapshot(&conn, &command_id);
    if committed {
        if after != snap {
            return Err(format!(
                "retry caused a second effect: before {snap:?} after {after:?}"
            ));
        }
        if number != pre_number {
            return Err(format!(
                "cached retry returned a different document number: {number} != {pre_number}"
            ));
        }
        let post_payload: Option<String> = conn
            .query_row(
                "SELECT response_payload FROM command_log WHERE command_id = ?1",
                [&command_id],
                |row| row.get(0),
            )
            .ok();
        if post_payload != pre_payload {
            return Err("command_log payload changed across retry".to_string());
        }
    } else {
        verify::v6_committed_effects(&conn, scenario.kind, &after)?;
    }
    verify::v1_integrity(&conn)?;
    verify::v2_audit_chain(&conn)?;
    verify::v3_reconciliation(&conn)?;
    verify::v4_gapless(&conn)?;
    drop(conn);
    // Temp dir drops here, after every child exit was confirmed above.
    Ok(expect_outcome.to_string())
}

/// CP-9 Test A: two racing checkouts, same product, stock for both.
fn run_concurrent_a() -> Result<String, String> {
    let dir = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let db = db_path(dir.path());
    driver::seed_db(&db);

    let mut first = spawn_child("--concurrent-child", "a0", &db);
    let mut second = spawn_child("--concurrent-child", "a1", &db);
    for (name, child) in [("a0", &mut first), ("a1", &mut second)] {
        match wait_bounded(child, CHILD_TIMEOUT_SECS) {
            Some(status) if status.success() => {}
            Some(status) => {
                let (_, stderr) = child_output(child);
                kill_and_reap(child);
                return Err(format!(
                    "concurrent child {name} failed ({status}): {stderr}"
                ));
            }
            None => {
                kill_and_reap(child);
                return Err(format!("concurrent child {name} timed out"));
            }
        }
    }

    let conn = verify::reopen(&db);
    verify::v1_integrity(&conn)?;
    verify::v2_audit_chain(&conn)?;
    verify::v3_reconciliation(&conn)?;
    let docs: i64 = conn
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .map_err(|e| format!("doc count: {e}"))?;
    if docs != 6 {
        return Err(format!("expected 6 committed documents, saw {docs}"));
    }
    let distinct_numbers: i64 = conn
        .query_row("SELECT COUNT(DISTINCT number) FROM documents", [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("distinct numbers: {e}"))?;
    if distinct_numbers != 6 {
        return Err("document numbers are not distinct".to_string());
    }
    verify::v4_gapless(&conn)?;
    // Each of the 6 sales carries its full effect set.
    let mut stmt = conn
        .prepare("SELECT id FROM documents")
        .map_err(|e| format!("doc list: {e}"))?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("doc rows: {e}"))?;
    for id in ids {
        let id = id.map_err(|e| format!("doc id: {e}"))?;
        let counts: (i64, i64, i64, i64, i64) = conn
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM document_items WHERE document_id = ?1),
                   (SELECT COUNT(*) FROM payments WHERE document_id = ?1),
                   (SELECT COUNT(*) FROM stock_movements WHERE reference_id = ?1),
                   (SELECT COUNT(*) FROM print_jobs WHERE document_id = ?1),
                   (SELECT COUNT(*) FROM cash_movements WHERE reference_id = ?1)",
                [&id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .map_err(|e| format!("effect set for {id}: {e}"))?;
        if counts != (1, 1, 1, 1, 1) {
            return Err(format!("incomplete effect set for {id}: {counts:?}"));
        }
    }
    let level: i64 = conn
        .query_row(
            "SELECT quantity_base_micro FROM stock_levels LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("final level: {e}"))?;
    let expected = scenario::SEED_LEVEL_MICRO - 6 * scenario::QTY_BASE_MICRO;
    if level != expected {
        return Err(format!("final stock {level} != seed-6 sales {expected}"));
    }
    let commands: i64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |row| row.get(0))
        .map_err(|e| format!("command count: {e}"))?;
    if commands != 6 {
        return Err(format!("expected 6 command rows, saw {commands}"));
    }
    Ok("both committed, 6/6 full effect sets".to_string())
}

/// CP-9 Test B: read-only observer racing a writing child.
fn run_concurrent_b() -> Result<String, String> {
    let dir = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let db = db_path(dir.path());
    driver::seed_db(&db);

    let mut writer = spawn_child("--concurrent-child", "b0", &db);
    let start = Instant::now();
    let cap = Duration::from_secs(POLLER_CAP_SECS);
    let mut reads = 0i64;
    loop {
        match writer.try_wait().map_err(|e| format!("try_wait: {e}"))? {
            Some(status) => {
                if !status.success() {
                    return Err(format!("writer child failed ({status})"));
                }
                break;
            }
            None => {
                if start.elapsed() >= cap {
                    kill_and_reap(&mut writer);
                    return Err("writer child exceeded poller cap".to_string());
                }
            }
        }
        // Parent-held read-only observation: must never see corruption
        // and must never block indefinitely (busy_timeout bounds it).
        // 1 ms poll: the writer finishes in tens of ms, so a tight poll
        // is what yields a meaningful read count (no sleeps added to the
        // writer; contention behavior is unperturbed).
        let conn = verify::reopen(&db);
        verify::v1_integrity(&conn).map_err(|e| format!("observer saw corruption: {e}"))?;
        reads += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
    if reads == 0 {
        return Err("observer performed zero reads".to_string());
    }
    Ok(format!("{reads} clean concurrent reads"))
}

fn parent_main() -> i32 {
    let start = Instant::now();
    let mut passed = 0i32;
    let mut total = 0i32;
    for scenario in SCENARIOS {
        total += 1;
        match run_scenario(scenario) {
            Ok(retry) => {
                passed += 1;
                println!("PASS {} (retry {retry})", scenario.id);
            }
            Err(diagnostic) => println!("FAIL {}: {diagnostic}", scenario.id),
        }
    }
    total += 1;
    match run_concurrent_a() {
        Ok(note) => {
            passed += 1;
            println!("PASS concurrent-a ({note})");
        }
        Err(diagnostic) => println!("FAIL concurrent-a: {diagnostic}"),
    }
    total += 1;
    match run_concurrent_b() {
        Ok(note) => {
            passed += 1;
            println!("PASS concurrent-b ({note})");
        }
        Err(diagnostic) => println!("FAIL concurrent-b: {diagnostic}"),
    }
    let elapsed = start.elapsed().as_secs();
    println!("{passed}/{total} scenarios passed in {elapsed}s");
    if passed == total {
        0
    } else {
        1
    }
}

fn child_main(args: &[String]) -> ! {
    let db = PathBuf::from(flag_value(args, "--db").expect("--db <path>"));
    if let Some(id) = flag_value(args, "--crash-child") {
        let scenario = SCENARIOS
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("unknown scenario {id}"));
        driver::crash_child(&id, scenario.kind, scenario.point, &db);
    }
    if let Some(id) = flag_value(args, "--retry-child") {
        let scenario = SCENARIOS
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("unknown scenario {id}"));
        driver::retry_child(&id, scenario.kind, &db);
    }
    if let Some(worker) = flag_value(args, "--concurrent-child") {
        driver::concurrent_child(&worker, &db);
    }
    eprintln!("crash_matrix_tests: unknown child mode");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if flag_value(&args, "--crash-child").is_some()
        || flag_value(&args, "--retry-child").is_some()
        || flag_value(&args, "--concurrent-child").is_some()
    {
        child_main(&args);
    }
    std::process::exit(parent_main());
}
