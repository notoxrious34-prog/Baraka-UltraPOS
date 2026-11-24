//! Sync use-case tests over in-memory port fakes (TASK-BRK-124).
//!
//! No SQLite here: the fakes return canned packages/outcomes so these tests
//! pin the authorization boundary (Gérant/Admin only — cashier fails
//! closed), the input validation (empty package, bad ids, bad limits), and
//! the outcome plumbing. The atomic SQLite merge ships separately.

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    SessionContext, SessionToken, SyncExportPort, SyncImportOutcome, SyncImportPort, SyncRenumber,
};
use baraka_application::use_cases::sync::{ExportSyncPackageUseCase, ImportSyncPackageUseCase};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use chrono::Utc;

fn admin_session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role: Role::Admin,
        expires_at: Utc::now() + chrono::TimeDelta::hours(1),
    }
}

fn cashier_session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role: Role::Caissier,
        expires_at: Utc::now() + chrono::TimeDelta::hours(1),
    }
}

struct FakeExports {
    package: String,
    count: usize,
}

impl SyncExportPort for FakeExports {
    fn export_package(
        &self,
        _terminal_id: &str,
        _since_created_at: &str,
        _limit: usize,
    ) -> Result<(String, usize), ApplicationError> {
        Ok((self.package.clone(), self.count))
    }
}

struct FakeImports {
    outcome: SyncImportOutcome,
}

impl SyncImportPort for FakeImports {
    fn import_package(
        &self,
        _package_text: &str,
        _warehouse_id: &str,
        _actor_user_id: &str,
        _terminal_id: &str,
        _now_rfc3339: &str,
    ) -> Result<SyncImportOutcome, ApplicationError> {
        Ok(self.outcome.clone())
    }
}

#[test]
fn export_requires_data_steward_role() {
    let fake = FakeExports {
        package: "HDR|SYNC1|x".to_string(),
        count: 1,
    };
    let use_case = ExportSyncPackageUseCase::new(&fake);
    let denied = use_case.execute(&cashier_session(), "T-1", "2026-01-01T00:00:00Z", 100);
    assert!(matches!(denied, Err(ApplicationError::Unauthorized)));
    let allowed = use_case
        .execute(&admin_session(), "T-1", "2026-01-01T00:00:00Z", 100)
        .expect("admin exports");
    assert_eq!(allowed.exported_docs, 1);
}

#[test]
fn export_rejects_bad_inputs_before_touching_the_port() {
    let fake = FakeExports {
        package: String::new(),
        count: 0,
    };
    let use_case = ExportSyncPackageUseCase::new(&fake);
    let session = admin_session();
    assert!(use_case
        .execute(&session, "", "2026-01-01T00:00:00Z", 100)
        .is_err());
    assert!(use_case.execute(&session, "T-1", "", 100).is_err());
    assert!(use_case
        .execute(&session, "T-1", "2026-01-01T00:00:00Z", 0)
        .is_err());
    assert!(use_case
        .execute(&session, "T-1", "2026-01-01T00:00:00Z", 5_001)
        .is_err());
}

#[test]
fn import_requires_data_steward_role_and_plumbs_outcome() {
    let outcome = SyncImportOutcome {
        imported_docs: 2,
        renumbered: vec![SyncRenumber {
            origin_number: "TICK-2026-000041".to_string(),
            new_number: "TICK-2026-000012".to_string(),
        }],
        negative_stock_warnings: vec![],
    };
    let fake = FakeImports { outcome };
    let use_case = ImportSyncPackageUseCase::new(&fake);
    let denied = use_case.execute(
        &cashier_session(),
        "HDR|SYNC1|x",
        "W-1",
        "T-9",
        "2026-01-01T00:00:00Z",
    );
    assert!(matches!(denied, Err(ApplicationError::Unauthorized)));
    let ok = use_case
        .execute(
            &admin_session(),
            "HDR|SYNC1|x",
            "W-1",
            "T-9",
            "2026-01-01T00:00:00Z",
        )
        .expect("admin imports");
    assert_eq!(ok.imported_docs, 2);
    assert_eq!(ok.renumbered[0].new_number, "TICK-2026-000012");
}

#[test]
fn import_rejects_empty_package_and_blank_ids() {
    let fake = FakeImports {
        outcome: SyncImportOutcome {
            imported_docs: 0,
            renumbered: vec![],
            negative_stock_warnings: vec![],
        },
    };
    let use_case = ImportSyncPackageUseCase::new(&fake);
    let session = admin_session();
    assert!(use_case
        .execute(&session, "   ", "W-1", "T-9", "now")
        .is_err());
    assert!(use_case
        .execute(&session, "HDR|x", "", "T-9", "now")
        .is_err());
    assert!(use_case
        .execute(&session, "HDR|x", "W-1", "", "now")
        .is_err());
}
