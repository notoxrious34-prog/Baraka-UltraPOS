//! Customer directory tests over real SQLite: CRUD, search, RBAC.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CustomerRepository, CustomerUpdate, NewCustomer, SessionContext, SessionToken,
};
use baraka_application::use_cases::customers::{
    CreateCustomerUseCase, GetCustomerUseCase, ListCustomersUseCase, SearchCustomersUseCase,
    UpdateCustomerUseCase,
};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Role;
use baraka_domain::sales::{CreditPolicy, CustomerId, CustomerTier};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCustomerRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

struct Harness {
    _dir: tempfile::TempDir,
    repo: SqliteCustomerRepository,
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-CUST";

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("cust.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    Harness {
        _dir: dir,
        repo: SqliteCustomerRepository::new(Arc::new(Mutex::new(conn))),
    }
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn disabled_input(name: &str, phone: Option<&str>) -> NewCustomer {
    NewCustomer {
        name: name.to_string(),
        phone: phone.map(str::to_string),
        address: None,
        credit_policy: CreditPolicy::Disabled,
        credit_limit_centimes: 0,
        customer_tier: CustomerTier::Retail,
    }
}

#[test]
fn create_valid_and_duplicate_phone_rejected() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateCustomerUseCase::new(&harness.repo);
    let id = create
        .execute(
            &gerant,
            disabled_input("Amine Benali", Some("0550123456")),
            TEST_TERMINAL,
        )
        .expect("create");
    let stored = harness.repo.get_customer(id).expect("lookup").expect("row");
    assert_eq!(stored.name, "Amine Benali");
    assert_eq!(stored.phone.as_deref(), Some("0550123456"));
    assert_eq!(stored.credit_policy, CreditPolicy::Disabled);

    let dup = create
        .execute(
            &gerant,
            disabled_input("Autre Nom", Some("0550123456")),
            TEST_TERMINAL,
        )
        .expect_err("duplicate phone");
    assert!(
        matches!(dup, ApplicationError::DuplicatePhone),
        "unexpected: {dup:?}"
    );
}

#[test]
fn invalid_phone_rejected_before_persistence() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateCustomerUseCase::new(&harness.repo);
    for bad in ["123", "058012345", "+213550123456", "05-50123456"] {
        let rejected = create
            .execute(
                &gerant,
                disabled_input("Nom Valide", Some(bad)),
                TEST_TERMINAL,
            )
            .expect_err("bad phone");
        assert!(
            matches!(
                rejected,
                ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
            ),
            "unexpected: {rejected:?}"
        );
    }
    assert!(harness.repo.list_customers(false).expect("list").is_empty());
}

#[test]
fn credit_policy_gating() {
    let harness = setup();
    let caissier = session_with(Role::Caissier);
    let gerant = session_with(Role::Gerant);
    let create = CreateCustomerUseCase::new(&harness.repo);
    let limited = NewCustomer {
        credit_policy: CreditPolicy::Limited,
        credit_limit_centimes: 50_000,
        customer_tier: CustomerTier::Retail,
        ..disabled_input("Limité", None)
    };
    // Caissier may create DISABLED customers but cannot enable credit.
    assert!(matches!(
        create.execute(&caissier, limited.clone(), TEST_TERMINAL),
        Err(ApplicationError::Unauthorized)
    ));
    // LIMITED without a positive limit is rejected even for Gerant.
    for bad_limit in [0, -100] {
        let rejected = create
            .execute(
                &gerant,
                NewCustomer {
                    credit_limit_centimes: bad_limit,
                    ..limited.clone()
                },
                TEST_TERMINAL,
            )
            .expect_err("bad limit");
        assert!(
            matches!(
                rejected,
                ApplicationError::Validation(DomainError::InvalidCreditLimit)
            ),
            "unexpected: {rejected:?}"
        );
    }
    let id = create
        .execute(&gerant, limited, TEST_TERMINAL)
        .expect("limited");
    let stored = harness.repo.get_customer(id).expect("lookup").expect("row");
    assert_eq!(stored.credit_limit_centimes, 50_000);
}

#[test]
fn update_paths_cover_rename_rephone_and_missing() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateCustomerUseCase::new(&harness.repo);
    let update = UpdateCustomerUseCase::new(&harness.repo);
    let id = create
        .execute(
            &gerant,
            disabled_input("Ancien Nom", Some("0550123456")),
            TEST_TERMINAL,
        )
        .expect("create");
    update
        .execute(
            &gerant,
            id,
            CustomerUpdate {
                name: Some("Nouveau Nom".to_string()),
                phone: Some(None),
                ..CustomerUpdate::default()
            },
        )
        .expect("update");
    let stored = harness.repo.get_customer(id).expect("lookup").expect("row");
    assert_eq!(stored.name, "Nouveau Nom");
    assert_eq!(stored.phone, None);

    // Re-registering the freed phone on another customer now succeeds.
    let other = create
        .execute(
            &gerant,
            disabled_input("Autre", Some("0550123456")),
            TEST_TERMINAL,
        )
        .expect("reuse freed phone");
    assert_ne!(other, id);

    // Unknown id surfaces CustomerNotFound.
    let ghost = CustomerId::new();
    assert!(matches!(
        update.execute(&gerant, ghost, CustomerUpdate::default()),
        Err(ApplicationError::CustomerNotFound)
    ));
}

#[test]
fn search_matches_name_phone_and_code() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateCustomerUseCase::new(&harness.repo);
    create
        .execute(
            &gerant,
            disabled_input("Amine Benali", Some("0550123456")),
            TEST_TERMINAL,
        )
        .expect("amine");
    create
        .execute(
            &gerant,
            disabled_input("Yacine Cherif", Some("0661987654")),
            TEST_TERMINAL,
        )
        .expect("yacine");
    let search = SearchCustomersUseCase::new(&harness.repo);
    let by_name = search.execute(&gerant, "amin", 20).expect("name");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].name, "Amine Benali");
    let by_phone = search.execute(&gerant, "6619", 20).expect("phone");
    assert_eq!(by_phone.len(), 1);
    assert_eq!(by_phone[0].name, "Yacine Cherif");
    // Case-insensitive ASCII match.
    let upper = search.execute(&gerant, "AMINE", 20).expect("upper");
    assert_eq!(upper.len(), 1);
    assert!(search
        .execute(&gerant, "   ", 20)
        .expect("blank")
        .is_empty());
    assert!(search.execute(&gerant, "zzz", 20).expect("miss").is_empty());
}

#[test]
fn list_respects_active_flag_and_rbac() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let vendeur = session_with(Role::Vendeur);
    let create = CreateCustomerUseCase::new(&harness.repo);
    let update = UpdateCustomerUseCase::new(&harness.repo);
    let id = create
        .execute(&gerant, disabled_input("Actif", None), TEST_TERMINAL)
        .expect("active");
    create
        .execute(&gerant, disabled_input("Second", None), TEST_TERMINAL)
        .expect("second");
    update
        .execute(
            &gerant,
            id,
            CustomerUpdate {
                is_active: Some(false),
                ..CustomerUpdate::default()
            },
        )
        .expect("deactivate");
    let list = ListCustomersUseCase::new(&harness.repo);
    assert_eq!(list.execute(&gerant, true).expect("active").len(), 1);
    assert_eq!(list.execute(&gerant, false).expect("all").len(), 2);
    // Vendeur reads freely but cannot create.
    assert_eq!(list.execute(&vendeur, true).expect("vendeur list").len(), 1);
    assert!(matches!(
        create.execute(&vendeur, disabled_input("X", None), TEST_TERMINAL),
        Err(ApplicationError::Unauthorized)
    ));
    let get = GetCustomerUseCase::new(&harness.repo);
    assert!(!get.execute(&vendeur, id).expect("vendeur get").is_active);
    assert!(matches!(
        get.execute(&vendeur, CustomerId::new()),
        Err(ApplicationError::CustomerNotFound)
    ));
}

#[test]
fn created_customer_persists_terminal_origin_device_id() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateCustomerUseCase::new(&harness.repo);
    let terminal = "TERMINAL-ORIGIN-001";
    let id = create
        .execute(
            &gerant,
            NewCustomer {
                name: "Origine Terminal".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Disabled,
                credit_limit_centimes: 0,
                customer_tier: CustomerTier::Retail,
            },
            terminal,
        )
        .expect("create");
    let stored: String = rusqlite::Connection::open(harness._dir.path().join("cust.db"))
        .expect("reopen")
        .query_row(
            "SELECT origin_device_id FROM customers WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .expect("origin");
    assert_eq!(stored, terminal);
}
