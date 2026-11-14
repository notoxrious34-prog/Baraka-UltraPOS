//! Credit overview tests over real SQLite (TASK-BRK-110): the aggregator
//! reads cached balances in one pass, classifies through the domain
//! risk bands, and returns the largest debtors first. RBAC enforced.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{CustomerRepository, NewCustomer, SessionContext, SessionToken};
use baraka_application::use_cases::customers::{CreateCustomerUseCase, GetCreditOverviewUseCase};
use baraka_domain::role::Role;
use baraka_domain::sales::{CreditPolicy, CreditRisk, CustomerTier};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCustomerRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

struct Harness {
    _dir: tempfile::TempDir,
    repo: SqliteCustomerRepository,
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-OVERVIEW";

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("overview.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    Harness {
        _dir: dir,
        repo: SqliteCustomerRepository::new(Arc::new(Mutex::new(conn))),
    }
}

fn session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role: Role::Gerant,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed(
    harness: &Harness,
    session: &SessionContext,
    name: &str,
    policy: CreditPolicy,
    limit: i64,
    balance: i64,
) {
    let id = CreateCustomerUseCase::new(&harness.repo)
        .execute(
            session,
            NewCustomer {
                name: name.to_string(),
                phone: None,
                address: None,
                credit_policy: policy,
                credit_limit_centimes: limit,
                customer_tier: CustomerTier::Retail,
            },
            TEST_TERMINAL,
        )
        .expect("create");
    harness
        .repo
        .update_cached_balance(id, balance)
        .expect("balance");
}

/// Five debtors + one clear customer:
/// - Karim: Limited 100.00, owes 150.00 → OVER_LIMIT
/// - Amine: Limited 100.00, owes 85.00 → NEAR_LIMIT
/// - Yacine: Limited 200.00, owes 10.00 → indebted
/// - Riad: Unlimited, owes 500.00 → indebted (never escalates)
/// - Nabil: Disabled, owes 5.00 → indebted (legacy balance, not over)
/// - Sara: Limited 100.00, owes 0 → excluded (not a debtor)
#[test]
fn overview_aggregates_bands_and_orders_largest_first() {
    let harness = setup();
    let session = session();
    seed(
        &harness,
        &session,
        "Karim",
        CreditPolicy::Limited,
        10_000,
        15_000,
    );
    seed(
        &harness,
        &session,
        "Amine",
        CreditPolicy::Limited,
        10_000,
        8_500,
    );
    seed(
        &harness,
        &session,
        "Yacine",
        CreditPolicy::Limited,
        20_000,
        1_000,
    );
    seed(
        &harness,
        &session,
        "Riad",
        CreditPolicy::Unlimited,
        0,
        50_000,
    );
    seed(&harness, &session, "Nabil", CreditPolicy::Disabled, 0, 500);
    seed(&harness, &session, "Sara", CreditPolicy::Limited, 10_000, 0);

    let overview = GetCreditOverviewUseCase::new(&harness.repo)
        .execute(&session, 10)
        .expect("overview");

    assert_eq!(overview.total_outstanding_centimes, 75_000);
    assert_eq!(overview.debtor_count, 5);
    assert_eq!(overview.over_limit_count, 1);
    assert_eq!(overview.near_limit_count, 1);
    assert_eq!(overview.top_debtors.len(), 5);
    // Largest first: Riad 500, Karim 150, Amine 85, Yacine 10, Nabil 5.
    let names: Vec<&str> = overview
        .top_debtors
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, vec!["Riad", "Karim", "Amine", "Yacine", "Nabil"]);
    assert_eq!(overview.top_debtors[1].risk, CreditRisk::OverLimit);
    assert_eq!(overview.top_debtors[2].risk, CreditRisk::NearLimit);
    assert_eq!(overview.top_debtors[0].risk, CreditRisk::Indebted);
}

#[test]
fn overview_cap_clamps_and_empty_book_is_zero() {
    let harness = setup();
    let session = session();
    let empty = GetCreditOverviewUseCase::new(&harness.repo)
        .execute(&session, 0)
        .expect("overview");
    assert_eq!(empty.total_outstanding_centimes, 0);
    assert_eq!(empty.debtor_count, 0);
    assert!(empty.top_debtors.is_empty());

    seed(
        &harness,
        &session,
        "Karim",
        CreditPolicy::Limited,
        10_000,
        15_000,
    );
    let capped = GetCreditOverviewUseCase::new(&harness.repo)
        .execute(&session, 1)
        .expect("overview");
    assert_eq!(capped.top_debtors.len(), 1);
    assert_eq!(capped.debtor_count, 1);
}
