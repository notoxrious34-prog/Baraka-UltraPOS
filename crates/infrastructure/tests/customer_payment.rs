//! Credit payment tests: balance math, overpayment policy, statements.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{CustomerRepository, SessionContext, SessionToken};
use baraka_application::use_cases::customers::{
    CreateCustomerUseCase, GetCustomerStatementUseCase, RecordCreditPaymentUseCase,
};
use baraka_domain::role::Role;
use baraka_domain::sales::{CreditPolicy, CustomerId};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCustomerRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "66666666-6666-6666-6666-666666666666";

struct Harness {
    _dir: tempfile::TempDir,
    repo: SqliteCustomerRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("pay.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('66666666-6666-6666-6666-666666666666', 'gerant1', 'hash', 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    Harness {
        _dir: dir,
        repo: SqliteCustomerRepository::new(Arc::new(Mutex::new(conn))),
    }
}

fn actor() -> UserId {
    UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid"))
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: actor(),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed_debtor(harness: &Harness, balance: i64) -> CustomerId {
    let gerant = session_with(Role::Gerant);
    let id = CreateCustomerUseCase::new(&harness.repo)
        .execute(
            &gerant,
            baraka_application::ports::NewCustomer {
                name: "Débiteur".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
            },
            TEST_TERMINAL,
        )
        .expect("customer");
    if balance > 0 {
        let doc = Uuid::new_v4();
        harness
            .repo
            .record_credit_debit(id, balance, balance, doc.into(), actor())
            .expect("debit");
        harness
            .repo
            .update_cached_balance(id, balance)
            .expect("balance");
    }
    id
}

fn ledger_count(harness: &Harness) -> i64 {
    rusqlite::Connection::open(harness._dir.path().join("pay.db"))
        .expect("reopen")
        .query_row("SELECT COUNT(*) FROM customer_ledger", [], |row| row.get(0))
        .expect("count")
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-PAY";

fn ensure_cash_session(harness: &Harness) {
    let conn = rusqlite::Connection::open(harness._dir.path().join("pay.db")).expect("reopen");
    conn.execute(
        "INSERT INTO warehouses (id, code, name, is_active, revision, origin_device_id, created_at, updated_at)
         VALUES ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'PAY-WH', 'Pay WH', 1, 1, 'LOCAL',
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("warehouse");
    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO cash_sessions
         (id, warehouse_id, terminal_id, status, opened_by, opened_at,
          opening_float_centimes, revision, origin_device_id, created_at, updated_at)
         VALUES (?1, 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', ?2, 'OPEN',
                 '66666666-6666-6666-6666-666666666666', '2026-01-01T00:00:00Z', 0, 1, ?2,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![session_id, TEST_TERMINAL,],
    )
    .expect("open session");
    conn.execute(
        "INSERT INTO cash_movements
         (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
         VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z',
                 '66666666-6666-6666-6666-666666666666')",
        rusqlite::params![Uuid::new_v4().to_string(), session_id,],
    )
    .expect("opening float");
}

#[test]
fn payment_reduces_balance_with_credit_row() {
    let harness = setup();
    ensure_cash_session(&harness);
    let id = seed_debtor(&harness, 20_000);
    let record = RecordCreditPaymentUseCase::new(&harness.repo)
        .execute(
            &session_with(Role::Caissier),
            id,
            8_000,
            Some("REC-1".to_string()),
            false,
            TEST_TERMINAL,
        )
        .expect("payment");
    assert_eq!(record.new_balance_centimes, 12_000);
    assert!(!record.ledger_entry_id.is_empty());
    let conn = rusqlite::Connection::open(harness._dir.path().join("pay.db")).expect("reopen");
    let row: (String, i64, i64, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT entry_type, amount_centimes, running_balance_centimes, document_type, document_id
             FROM customer_ledger WHERE id = ?1",
            [&record.ledger_entry_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .expect("row");
    assert_eq!(row.0, "CREDIT");
    assert_eq!((row.1, row.2), (8_000, 12_000));
    assert_eq!(row.3.as_deref(), Some("PAYMENT"));
    assert_eq!(row.4.as_deref(), Some("REC-1"));
    let balance: i64 = conn
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .expect("balance");
    assert_eq!(balance, 12_000);
}

#[test]
fn overpayment_without_flag_rejected_with_zero_writes() {
    let harness = setup();
    ensure_cash_session(&harness);
    let id = seed_debtor(&harness, 5_000);
    let before = ledger_count(&harness);
    let rejected = RecordCreditPaymentUseCase::new(&harness.repo)
        .execute(
            &session_with(Role::Gerant),
            id,
            8_000,
            None,
            false,
            TEST_TERMINAL,
        )
        .expect_err("overpay");
    assert!(
        matches!(
            rejected,
            ApplicationError::PaymentExceedsBalance {
                balance_centimes: 5_000,
                attempted_centimes: 8_000,
            }
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(ledger_count(&harness), before);
}

#[test]
fn overpayment_with_flag_needs_credit_authority() {
    let harness = setup();
    ensure_cash_session(&harness);
    let id = seed_debtor(&harness, 5_000);
    // Caissier lacks ManageCustomerCredit: rejected before any write.
    let denied = RecordCreditPaymentUseCase::new(&harness.repo)
        .execute(
            &session_with(Role::Caissier),
            id,
            8_000,
            None,
            true,
            TEST_TERMINAL,
        )
        .expect_err("caissier overpay");
    assert!(
        matches!(denied, ApplicationError::OverpaymentUnauthorized),
        "unexpected: {denied:?}"
    );
    assert_eq!(ledger_count(&harness), 1);
    // Gerant holds it: negative balance permitted and recorded.
    let record = RecordCreditPaymentUseCase::new(&harness.repo)
        .execute(
            &session_with(Role::Gerant),
            id,
            8_000,
            None,
            true,
            TEST_TERMINAL,
        )
        .expect("gerant overpay");
    assert_eq!(record.new_balance_centimes, -3_000);
}

#[test]
fn statement_running_balances_match_final_balance() {
    let harness = setup();
    ensure_cash_session(&harness);
    let id = seed_debtor(&harness, 0);
    let gerant = session_with(Role::Gerant);
    let pay = RecordCreditPaymentUseCase::new(&harness.repo);
    // Build: DEBIT 20000 -> CREDIT 8000 -> DEBIT 5000 -> CREDIT 2000.
    let doc = Uuid::new_v4();
    harness
        .repo
        .record_credit_debit(id, 20_000, 20_000, doc.into(), actor())
        .expect("debit1");
    harness
        .repo
        .update_cached_balance(id, 20_000)
        .expect("bal1");
    pay.execute(&gerant, id, 8_000, None, false, TEST_TERMINAL)
        .expect("pay1");
    let doc2 = Uuid::new_v4();
    harness
        .repo
        .record_credit_debit(id, 5_000, 17_000, doc2.into(), actor())
        .expect("debit2");
    harness
        .repo
        .update_cached_balance(id, 17_000)
        .expect("bal2");
    pay.execute(&gerant, id, 2_000, None, false, TEST_TERMINAL)
        .expect("pay2");

    let rows = GetCustomerStatementUseCase::new(&harness.repo)
        .execute(&gerant, id, None, None)
        .expect("statement");
    assert_eq!(rows.len(), 4);
    let pairs: Vec<(&str, i64, i64)> = rows
        .iter()
        .map(|row| {
            (
                row.entry_type.as_str(),
                row.amount_centimes,
                row.balance_after_centimes,
            )
        })
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("DEBIT", 20_000, 20_000),
            ("CREDIT", 8_000, 12_000),
            ("DEBIT", 5_000, 17_000),
            ("CREDIT", 2_000, 15_000),
        ]
    );
    // Final running balance equals the cached customer balance.
    let profile = harness
        .repo
        .get_credit_profile(id)
        .expect("profile")
        .expect("row");
    assert_eq!(profile.current_balance_centimes, 15_000);
    assert_eq!(pairs[3].2, profile.current_balance_centimes);
}

#[test]
fn invalid_amounts_and_vendeur_rejected() {
    let harness = setup();
    ensure_cash_session(&harness);
    let id = seed_debtor(&harness, 10_000);
    let before = ledger_count(&harness);
    let pay = RecordCreditPaymentUseCase::new(&harness.repo);
    for bad in [0, -500] {
        assert!(
            pay.execute(
                &session_with(Role::Caissier),
                id,
                bad,
                None,
                false,
                TEST_TERMINAL
            )
            .is_err(),
            "amount {bad} must fail"
        );
    }
    assert!(matches!(
        pay.execute(
            &session_with(Role::Vendeur),
            id,
            1_000,
            None,
            false,
            TEST_TERMINAL
        ),
        Err(ApplicationError::Unauthorized)
    ));
    assert_eq!(ledger_count(&harness), before);
}
