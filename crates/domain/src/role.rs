//! RBAC roles and permissions (Architecture Section 25).
//!
//! Exactly four roles in a strictly hierarchical additive matrix.
//! No per-user custom overrides exist in this task.

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;

/// Authenticated operator role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    /// Counter sales only.
    Vendeur,
    /// Vendeur + cash collection and cash sessions.
    Caissier,
    /// Caissier + overrides, returns, stock, purchases, reports.
    Gerant,
    /// Gerant + users, settings, backup/restore.
    Admin,
}

/// Single authorizable capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Permission {
    PosSell,
    PosParkResume,
    CashierCollectPayment,
    CashSessionOpen,
    CashSessionClose,
    PriceOverride,
    DiscountOverride,
    NegativeStockOverride,
    CreditLimitOverride,
    ProcessReturn,
    StockAdjustment,
    PurchaseManage,
    ReportsView,
    UserManage,
    SystemSettingsManage,
    BackupRestoreManage,
    /// Read warehouses and stock levels (all roles).
    ViewStock,
    /// Manual stock corrections (Gerant, Admin).
    AdjustStock,
    /// Warehouse-to-warehouse transfers (Gerant, Admin).
    TransferStock,
    /// Warehouse master-data management (Admin only).
    ManageWarehouses,
    /// Checkout and draft sales (all roles).
    CreateSale,
    /// Read customer directory (all roles).
    ViewCustomers,
    /// Create customers (Caissier, Gerant, Admin).
    CreateCustomer,
    /// Credit policy/limit management + overpayment approval (Gerant, Admin).
    ManageCustomerCredit,
    /// Record customer credit payments (Caissier, Gerant, Admin).
    RecordCreditPayment,
    /// Enqueue reprint jobs (Caissier, Gerant, Admin; NOT Vendeur).
    ReprintDocument,
    /// Printer transport configuration (Admin only).
    ManagePrinterSettings,
    /// Record drawer expenses (Caissier, Gerant, Admin).
    CashExpenseRecord,
    /// Read cash session reports / Z-reports (Caissier, Gerant, Admin).
    CashReportView,
    /// Supplier master-data management (Gerant, Admin only).
    SupplierManage,
    /// Create/edit purchase orders BC (Gerant, Admin only).
    PurchaseOrderCreate,
    /// Receive goods BR (Gerant, Admin only).
    PurchaseReceive,
    /// Record supplier invoices FF (Gerant, Admin only).
    PurchaseInvoiceRecord,
    /// Record supplier payments CR (Gerant, Admin only).
    SupplierPaymentRecord,
}

impl Role {
    /// Stable wire/DB code for this role.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Vendeur => "VENDEUR",
            Self::Caissier => "CAISSIER",
            Self::Gerant => "GERANT",
            Self::Admin => "ADMIN",
        }
    }

    /// Parse a stored role code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "VENDEUR" => Ok(Self::Vendeur),
            "CAISSIER" => Ok(Self::Caissier),
            "GERANT" => Ok(Self::Gerant),
            "ADMIN" => Ok(Self::Admin),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown role code: {other}"
            ))),
        }
    }

    /// Exact permission set for this role (additive hierarchy).
    #[must_use]
    pub const fn permissions(self) -> &'static [Permission] {
        use Permission::{
            AdjustStock, BackupRestoreManage, CashExpenseRecord, CashReportView, CashSessionClose,
            CashSessionOpen, CashierCollectPayment, CreateCustomer, CreateSale,
            CreditLimitOverride, DiscountOverride, ManageCustomerCredit, ManagePrinterSettings,
            ManageWarehouses, NegativeStockOverride, PosParkResume, PosSell, PriceOverride,
            ProcessReturn, PurchaseInvoiceRecord, PurchaseManage, PurchaseOrderCreate,
            PurchaseReceive, RecordCreditPayment, ReportsView, ReprintDocument, StockAdjustment,
            SupplierManage, SupplierPaymentRecord, SystemSettingsManage, TransferStock, UserManage,
            ViewCustomers, ViewStock,
        };
        match self {
            Self::Vendeur => &[PosSell, PosParkResume, ViewStock, CreateSale, ViewCustomers],
            Self::Caissier => &[
                PosSell,
                PosParkResume,
                CashierCollectPayment,
                CashSessionOpen,
                CashSessionClose,
                CashExpenseRecord,
                CashReportView,
                ViewStock,
                CreateSale,
                ViewCustomers,
                CreateCustomer,
                RecordCreditPayment,
                ReprintDocument,
            ],
            Self::Gerant => &[
                PosSell,
                PosParkResume,
                CashierCollectPayment,
                CashSessionOpen,
                CashSessionClose,
                CashExpenseRecord,
                CashReportView,
                PriceOverride,
                DiscountOverride,
                NegativeStockOverride,
                CreditLimitOverride,
                ProcessReturn,
                StockAdjustment,
                PurchaseManage,
                ReportsView,
                ViewStock,
                AdjustStock,
                TransferStock,
                CreateSale,
                ViewCustomers,
                CreateCustomer,
                ManageCustomerCredit,
                RecordCreditPayment,
                ReprintDocument,
                SupplierManage,
                PurchaseOrderCreate,
                PurchaseReceive,
                PurchaseInvoiceRecord,
                SupplierPaymentRecord,
            ],
            Self::Admin => &[
                PosSell,
                PosParkResume,
                CashierCollectPayment,
                CashSessionOpen,
                CashSessionClose,
                CashExpenseRecord,
                CashReportView,
                PriceOverride,
                DiscountOverride,
                NegativeStockOverride,
                CreditLimitOverride,
                ProcessReturn,
                StockAdjustment,
                PurchaseManage,
                ReportsView,
                UserManage,
                SystemSettingsManage,
                BackupRestoreManage,
                ViewStock,
                AdjustStock,
                TransferStock,
                ManageWarehouses,
                CreateSale,
                ViewCustomers,
                CreateCustomer,
                ManageCustomerCredit,
                RecordCreditPayment,
                ReprintDocument,
                ManagePrinterSettings,
                SupplierManage,
                PurchaseOrderCreate,
                PurchaseReceive,
                PurchaseInvoiceRecord,
                SupplierPaymentRecord,
            ],
        }
    }

    /// Whether this role grants `permission`.
    #[must_use]
    pub fn has_permission(self, permission: Permission) -> bool {
        self.permissions().contains(&permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Permission; 34] = [
        Permission::PosSell,
        Permission::PosParkResume,
        Permission::CashierCollectPayment,
        Permission::CashSessionOpen,
        Permission::CashSessionClose,
        Permission::CashExpenseRecord,
        Permission::CashReportView,
        Permission::PriceOverride,
        Permission::DiscountOverride,
        Permission::NegativeStockOverride,
        Permission::CreditLimitOverride,
        Permission::ProcessReturn,
        Permission::StockAdjustment,
        Permission::PurchaseManage,
        Permission::ReportsView,
        Permission::UserManage,
        Permission::SystemSettingsManage,
        Permission::BackupRestoreManage,
        Permission::ViewStock,
        Permission::AdjustStock,
        Permission::TransferStock,
        Permission::ManageWarehouses,
        Permission::CreateSale,
        Permission::ViewCustomers,
        Permission::CreateCustomer,
        Permission::ManageCustomerCredit,
        Permission::RecordCreditPayment,
        Permission::ReprintDocument,
        Permission::ManagePrinterSettings,
        Permission::SupplierManage,
        Permission::PurchaseOrderCreate,
        Permission::PurchaseReceive,
        Permission::PurchaseInvoiceRecord,
        Permission::SupplierPaymentRecord,
    ];

    #[test]
    fn matrix_matches_constitution_for_all_roles_and_permissions() {
        let expectations: [(Role, &[Permission]); 4] = [
            (
                Role::Vendeur,
                &[
                    Permission::PosSell,
                    Permission::PosParkResume,
                    Permission::ViewStock,
                    Permission::CreateSale,
                    Permission::ViewCustomers,
                ],
            ),
            (
                Role::Caissier,
                &[
                    Permission::PosSell,
                    Permission::PosParkResume,
                    Permission::CashierCollectPayment,
                    Permission::CashSessionOpen,
                    Permission::CashSessionClose,
                    Permission::CashExpenseRecord,
                    Permission::CashReportView,
                    Permission::ViewStock,
                    Permission::CreateSale,
                    Permission::ViewCustomers,
                    Permission::CreateCustomer,
                    Permission::RecordCreditPayment,
                    Permission::ReprintDocument,
                ],
            ),
            (
                Role::Gerant,
                &[
                    Permission::PosSell,
                    Permission::PosParkResume,
                    Permission::CashierCollectPayment,
                    Permission::CashSessionOpen,
                    Permission::CashSessionClose,
                    Permission::CashExpenseRecord,
                    Permission::CashReportView,
                    Permission::PriceOverride,
                    Permission::DiscountOverride,
                    Permission::NegativeStockOverride,
                    Permission::CreditLimitOverride,
                    Permission::ProcessReturn,
                    Permission::StockAdjustment,
                    Permission::PurchaseManage,
                    Permission::ReportsView,
                    Permission::ViewStock,
                    Permission::AdjustStock,
                    Permission::TransferStock,
                    Permission::CreateSale,
                    Permission::ViewCustomers,
                    Permission::CreateCustomer,
                    Permission::ManageCustomerCredit,
                    Permission::RecordCreditPayment,
                    Permission::ReprintDocument,
                    Permission::SupplierManage,
                    Permission::PurchaseOrderCreate,
                    Permission::PurchaseReceive,
                    Permission::PurchaseInvoiceRecord,
                    Permission::SupplierPaymentRecord,
                ],
            ),
            (Role::Admin, &ALL),
        ];
        for (role, expected) in expectations {
            assert_eq!(role.permissions(), expected, "set for {role:?}");
            // 4 roles x 34 permissions = 136 assertions against the matrix.
            for permission in ALL {
                assert_eq!(
                    role.has_permission(permission),
                    expected.contains(&permission),
                    "{role:?} x {permission:?}"
                );
            }
        }
        assert_eq!(Role::Vendeur.permissions().len(), 5);
        assert_eq!(Role::Caissier.permissions().len(), 13);
        assert_eq!(Role::Gerant.permissions().len(), 29);
        assert_eq!(Role::Admin.permissions().len(), 34);
        // Pre-existing mappings are untouched by later extensions.
        assert!(!Role::Vendeur.has_permission(Permission::AdjustStock));
        assert!(!Role::Vendeur.has_permission(Permission::ReprintDocument));
        assert!(!Role::Caissier.has_permission(Permission::TransferStock));
        assert!(!Role::Caissier.has_permission(Permission::ManagePrinterSettings));
        assert!(!Role::Gerant.has_permission(Permission::ManageWarehouses));
        assert!(!Role::Gerant.has_permission(Permission::ManagePrinterSettings));
        assert!(Role::Admin.has_permission(Permission::ManageWarehouses));
        assert!(Role::Admin.has_permission(Permission::ManagePrinterSettings));
        // Supplier/purchase permissions are strictly Gerant + Admin.
        for permission in [
            Permission::SupplierManage,
            Permission::PurchaseOrderCreate,
            Permission::PurchaseReceive,
            Permission::PurchaseInvoiceRecord,
            Permission::SupplierPaymentRecord,
        ] {
            assert!(
                !Role::Vendeur.has_permission(permission),
                "Vendeur x {permission:?}"
            );
            assert!(
                !Role::Caissier.has_permission(permission),
                "Caissier x {permission:?}"
            );
            assert!(
                Role::Gerant.has_permission(permission),
                "Gerant x {permission:?}"
            );
            assert!(
                Role::Admin.has_permission(permission),
                "Admin x {permission:?}"
            );
        }
    }

    #[test]
    fn role_codes_round_trip() {
        for role in [Role::Vendeur, Role::Caissier, Role::Gerant, Role::Admin] {
            assert_eq!(Role::from_code(role.code()), Ok(role));
        }
        assert!(Role::from_code("SUPERUSER").is_err());
    }
}
