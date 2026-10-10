//! Outbound ports (interfaces the infrastructure layer must implement).
//!
//! Authentication ports (password hashing, user lookup, volatile sessions)
//! join the original clock/lookup contracts. The dependency direction stays
//! Application <- Infrastructure. The canonical system clock implementation
//! lives in infrastructure (`system_clock.rs`); only the trait lives here.

use baraka_domain::catalog::{
    Barcode, Category, CategoryId, KitItem, Product, ProductId, ProductUnit, Sku, UnitId,
};
use baraka_domain::inventory::{
    ReferenceType, StockMovementType, Warehouse, WarehouseCode, WarehouseId,
};
use baraka_domain::primitives::FactorMicro;
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CreditProfile, Customer, CustomerId, Document, DocumentId, DocumentItem,
    DocumentLanguage,
};
use baraka_domain::user::{User, UserId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::errors::ApplicationError;

/// Clock port: single source of time for use cases and session expiry.
pub trait Clock: Send + Sync {
    /// Current UTC instant.
    fn now(&self) -> DateTime<Utc>;

    /// Current UTC time as RFC3339 millis string.
    fn now_rfc3339(&self) -> String;
}

/// Product lookup port used by the fast POS search path.
pub trait ProductLookupPort: Send + Sync {
    /// Returns true when the lookup backend is reachable and indexed.
    fn is_ready(&self) -> bool;
}

/// Secret hashing port (Argon2id in infrastructure; fakes in tests).
pub trait PasswordHasher: Send + Sync {
    /// Hash a plaintext secret (fresh random salt per call).
    fn hash(&self, plaintext: &str) -> Result<String, ApplicationError>;

    /// Returns true when `plaintext` matches the stored PHC hash.
    fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, ApplicationError>;
}

/// User persistence port (SQLite repository in infrastructure).
pub trait UserRepository: Send + Sync {
    /// Look up an operator by login name.
    fn find_by_username(&self, username: &str) -> Result<Option<User>, ApplicationError>;

    /// Increment the failure counter and record a new lock (or none).
    fn record_login_failure(
        &self,
        user_id: UserId,
        locked_until: Option<DateTime<Utc>>,
    ) -> Result<(), ApplicationError>;

    /// Clear failures and any recorded lock after a successful login.
    fn reset_login_failures(&self, user_id: UserId) -> Result<(), ApplicationError>;

    /// Stored password hash slot for the user.
    fn get_password_hash(&self, user_id: UserId) -> Result<String, ApplicationError>;

    /// Stored PIN hash slot, if a PIN was ever configured.
    fn get_pin_hash(&self, user_id: UserId) -> Result<Option<String>, ApplicationError>;

    /// Total operator rows (read-only first-run probe; no side effects).
    fn count_users(&self) -> Result<u64, ApplicationError>;

    /// Atomically create the very first Admin operator.
    ///
    /// The implementation MUST run inside ONE `BEGIN IMMEDIATE` transaction
    /// and re-check, inside that transaction, that the `users` table is
    /// still empty; a non-empty table yields
    /// `DomainError::BootstrapAlreadyCompleted` and zero inserted rows.
    /// The audit event travels in the same transaction.
    fn bootstrap_first_admin(
        &self,
        command: &BootstrapFirstAdminCommand,
    ) -> Result<UserId, ApplicationError>;
}

/// Atomic first-admin creation command (pre-validated by the use case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapFirstAdminCommand {
    /// Fresh operator id allocated by the use case.
    pub user_id: UserId,
    /// Validated, trimmed login name.
    pub username: String,
    /// Argon2id PHC hash produced by the shared [`PasswordHasher`] port.
    pub password_hash: String,
    /// Capture instant (RFC3339 millis) for `created_at`/`updated_at`/audit.
    pub created_at: String,
    /// Terminal identifier resolved from TerminalContext (never client input).
    pub terminal_id: String,
}

/// Opaque volatile session token (UUID v4; local-desktop scope only).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionToken(pub String);

impl SessionToken {
    /// Mint a fresh random token.
    #[must_use]
    pub fn new_random() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Borrow the opaque value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated live session handed to authorized callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    /// Token this context was validated for.
    pub token: SessionToken,
    /// Authenticated operator.
    pub user_id: UserId,
    /// Operator role at login time.
    pub role: Role,
    /// Fixed expiry instant (no sliding refresh).
    pub expires_at: DateTime<Utc>,
}

/// Volatile session port (in-memory store only; never persisted).
pub trait SessionStore: Send + Sync {
    /// Create a session living `ttl_seconds` from now.
    fn create_session(&self, user_id: UserId, role: Role, ttl_seconds: i64) -> SessionToken;

    /// Return the live context, or `None` when unknown or expired.
    fn validate_session(&self, token: &SessionToken) -> Option<SessionContext>;

    /// Revoke a session; unknown tokens are a silent no-op.
    fn invalidate_session(&self, token: &SessionToken);
}

/// Catalog persistence port (SQLite repository in infrastructure).
pub trait ProductRepository: Send + Sync {
    /// Persist a product with its full unit set atomically.
    /// `origin_device_id` is the TerminalContext-resolved terminal identifier.
    fn create_product(
        &self,
        product: &Product,
        units: &[ProductUnit],
        origin_device_id: &str,
    ) -> Result<(), ApplicationError>;

    /// Update product metadata and upsert its unit set atomically.
    fn update_product(
        &self,
        product: &Product,
        units: &[ProductUnit],
    ) -> Result<(), ApplicationError>;

    /// Fetch a product with all its units.
    fn find_by_id(
        &self,
        id: ProductId,
    ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError>;

    /// Resolve a barcode to its exact unit and parent product.
    fn find_by_barcode(
        &self,
        barcode: &Barcode,
    ) -> Result<Option<(Product, ProductUnit)>, ApplicationError>;

    /// Fetch a product with all its units by SKU.
    fn find_by_sku(
        &self,
        sku: &Sku,
    ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError>;

    /// Replace the kit's component graph atomically.
    fn define_kit(&self, kit_id: ProductId, items: &[KitItem]) -> Result<(), ApplicationError>;

    /// Stored BOM lines for a kit header.
    fn get_kit_items(&self, kit_id: ProductId) -> Result<Vec<KitItem>, ApplicationError>;

    /// Every stored kit edge (header, component) for cycle validation.
    fn get_all_kit_edges(&self) -> Result<Vec<(ProductId, ProductId)>, ApplicationError>;

    /// Prefix/multilingual search capped at `limit` products.
    fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(Product, Vec<ProductUnit>)>, ApplicationError>;

    /// Direct BOM rows for a kit header (no recursion; used by expansion).
    fn get_direct_kit_items(
        &self,
        kit_product_id: ProductId,
    ) -> Result<Vec<KitComponentRow>, ApplicationError>;
}

/// Category persistence port (SQLite repository in infrastructure).
pub trait CategoryRepository: Send + Sync {
    /// Persist a category.
    fn create_category(&self, category: &Category) -> Result<(), ApplicationError>;

    /// Fetch a category by id.
    fn find_category_by_id(&self, id: CategoryId) -> Result<Option<Category>, ApplicationError>;

    /// List all categories.
    fn list_categories(&self) -> Result<Vec<Category>, ApplicationError>;
}

/// One direct BOM edge (header id carried by the call, not the row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitComponentRow {
    /// Component product.
    pub component_product_id: ProductId,
    /// Component unit the quantity is expressed in.
    pub unit_id: UnitId,
    /// Quantity per kit base unit, in unit micros.
    pub quantity_micro: i64,
}

/// Warehouse persistence port (SQLite repository in infrastructure).
pub trait WarehouseRepository: Send + Sync {
    /// Persist a warehouse. `origin_device_id` is TerminalContext-resolved.
    fn create_warehouse(
        &self,
        warehouse: &Warehouse,
        origin_device_id: &str,
    ) -> Result<(), ApplicationError>;

    /// Fetch a warehouse by id.
    fn get_by_id(&self, id: WarehouseId) -> Result<Option<Warehouse>, ApplicationError>;

    /// Fetch a warehouse by business code.
    fn get_by_code(&self, code: &WarehouseCode) -> Result<Option<Warehouse>, ApplicationError>;

    /// List active warehouses ordered by code.
    fn list_active(&self) -> Result<Vec<Warehouse>, ApplicationError>;
}

/// Atomic stock-movement command (single ledger line + level update).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockMovementCommand {
    /// Physical product (kits rejected by the engine).
    pub product_id: ProductId,
    /// Warehouse where the movement lands.
    pub warehouse_id: WarehouseId,
    /// Handled selling unit.
    pub unit_id: UnitId,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: i64,
    /// Direction + business meaning.
    pub movement_type: StockMovementType,
    /// Referenced document kind, if any.
    pub reference_type: Option<ReferenceType>,
    /// Referenced document id, if any.
    pub reference_id: Option<String>,
    /// Shared id of a transfer pair, if any.
    pub transfer_batch_id: Option<String>,
    /// Allow the level to go negative (transfers always refuse).
    pub authorize_negative_stock: bool,
    /// Human reason (mandatory for adjustments).
    pub reason: Option<String>,
    /// Acting operator.
    pub actor: UserId,
}

/// New movement row for direct appends (granular seam; the atomic engine
/// inlines its own insert to share the write transaction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStockMovement {
    /// Physical product.
    pub product_id: ProductId,
    /// Target warehouse.
    pub warehouse_id: WarehouseId,
    /// Handled selling unit.
    pub unit_id: UnitId,
    /// Direction + business meaning.
    pub movement_type: StockMovementType,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: i64,
    /// Positive magnitude in base-unit micros.
    pub quantity_base_micro: i64,
    /// Referenced document kind, if any.
    pub reference_type: Option<ReferenceType>,
    /// Referenced document id, if any.
    pub reference_id: Option<String>,
    /// Shared id of a transfer pair, if any.
    pub transfer_batch_id: Option<String>,
    /// Override flag as stored.
    pub authorized_negative_stock: bool,
    /// Human reason, if any.
    pub reason: Option<String>,
    /// Acting operator.
    pub actor: UserId,
}

/// Persisted movement outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockMovementRecord {
    /// Stored movement id (TEXT UUID).
    pub movement_id: String,
    /// Level after the movement, in base micros.
    pub new_quantity_base_micro: i64,
    /// Whether the authorized-negative path was taken (audit written).
    pub negative_override_applied: bool,
}

/// Atomic warehouse-to-warehouse transfer command (OUT + IN pair).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferCommand {
    /// Physical product (kits rejected by the engine).
    pub product_id: ProductId,
    /// Source warehouse (must hold the stock).
    pub source_warehouse_id: WarehouseId,
    /// Destination warehouse.
    pub destination_warehouse_id: WarehouseId,
    /// Handled selling unit.
    pub unit_id: UnitId,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: i64,
    /// Shared batch id for the pair.
    pub transfer_batch_id: String,
    /// Optional human reason.
    pub reason: Option<String>,
    /// Acting operator.
    pub actor: UserId,
}

/// Persisted transfer outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferResult {
    /// Shared batch id.
    pub transfer_batch_id: String,
    /// Stored OUT movement id.
    pub out_movement_id: String,
    /// Stored IN movement id.
    pub in_movement_id: String,
    /// Source level after the movement.
    pub source_new_quantity_base_micro: i64,
    /// Destination level after the movement.
    pub destination_new_quantity_base_micro: i64,
}

/// Stock persistence port (SQLite repository in infrastructure).
///
/// `apply_stock_movement` and `transfer_stock` each execute their full
/// read-modify-write cycle inside ONE `run_transaction` call; the granular
/// methods below expose the same storage for queries and direct tests.
pub trait StockRepository: Send + Sync {
    /// Current level in base micros; 0 when no row exists.
    fn get_stock_level(
        &self,
        product_id: ProductId,
        warehouse_id: WarehouseId,
    ) -> Result<i64, ApplicationError>;

    /// Add a signed delta to a level row, creating it at zero first when
    /// absent. Returns the new quantity.
    fn upsert_stock_level_with_delta(
        &self,
        product_id: ProductId,
        warehouse_id: WarehouseId,
        signed_delta_base_micro: i64,
    ) -> Result<i64, ApplicationError>;

    /// Append one movement row; returns its stored id.
    fn insert_stock_movement(
        &self,
        movement: &NewStockMovement,
    ) -> Result<String, ApplicationError>;

    /// Conversion factor of a unit (micros per selling unit).
    fn get_product_unit_conversion(
        &self,
        product_id: ProductId,
        unit_id: UnitId,
    ) -> Result<FactorMicro, ApplicationError>;

    /// Execute the full atomic movement algorithm (fetch-or-create level,
    /// revision-guarded update, movement insert, conditional audit).
    fn apply_stock_movement(
        &self,
        command: &StockMovementCommand,
    ) -> Result<StockMovementRecord, ApplicationError>;

    /// Execute a transfer OUT/IN pair inside ONE transaction.
    fn transfer_stock(&self, command: &TransferCommand)
        -> Result<TransferResult, ApplicationError>;
}

/// Single-purpose audit seam for authorized negative-stock overrides.
///
/// The hash-chained `audit_log` insert itself runs inside the engine's own
/// transaction for atomicity; this port exposes the same event for direct
/// use and future engines.
pub trait AuditPort: Send + Sync {
    /// Record one authorized negative-stock override event.
    fn record_negative_stock_override(
        &self,
        product_id: ProductId,
        warehouse_id: WarehouseId,
        resulting_quantity_base_micro: i64,
        actor: UserId,
        at: DateTime<Utc>,
    ) -> Result<(), ApplicationError>;
}

/// Fully priced sale plan handed to the atomic checkout (all i64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutCommand {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Canonical request hash input built by the use case.
    pub request_canonical: String,
    /// Selling warehouse.
    pub warehouse_id: WarehouseId,
    /// Terminal identifier resolved from TerminalContext (never client input).
    pub terminal_id: String,
    /// Customer for credit sales, if any.
    pub customer_id: Option<CustomerId>,
    /// Document kind code (`TICKET`, `FACTURE`, `BL`).
    pub doc_type_code: String,
    /// Sale-time receipt language (explicit, never defaulted).
    pub document_language: DocumentLanguage,
    /// Priced cart lines in display order.
    pub priced_lines: Vec<PricedCheckoutLine>,
    /// Leaf stock deductions (expanded kits + direct lines).
    pub leaf_deductions: Vec<LeafDeduction>,
    /// Tender lines.
    pub payments: Vec<CheckoutPayment>,
    /// Priced totals.
    pub totals: CheckoutTotals,
    /// Credit amount covered by CREDIT lines (0 when none).
    pub credit_amount_centimes: i64,
    /// Actor holds CreditLimitOverride (authoritative re-check in txn).
    pub credit_override_granted: bool,
    /// Operator explicitly confirmed sale-on-shortage in this session
    /// (TASK-BRK-033). Only meaningful together with
    /// `negative_override_permitted`; never grants anything by itself.
    pub allow_negative_stock: bool,
    /// Actor holds NegativeStockOverride (authoritative in-txn gate).
    pub negative_override_permitted: bool,
    /// Acting operator.
    pub actor: UserId,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// One priced cart line for persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricedCheckoutLine {
    /// Product sold (leaf or kit header as displayed).
    pub product_id: ProductId,
    /// Selling unit handled.
    pub unit_id: UnitId,
    /// Quantity in selling-unit micros.
    pub quantity_micro: i64,
    /// Quantity converted to base micros.
    pub quantity_base_micro: i64,
    /// Agreed unit price in centimes.
    pub unit_price_centimes: i64,
    /// Line discount in centimes.
    pub discount_centimes: i64,
    /// TVA rate in micros (basis points x 100).
    pub tva_rate_micro: i64,
    /// Line HT after all discounts.
    pub line_ht_centimes: i64,
    /// Line TVA.
    pub line_tva_centimes: i64,
    /// Line TTC.
    pub line_ttc_centimes: i64,
    /// Purchase price snapshot (margin base).
    pub cost_price_centimes: i64,
    /// Display order.
    pub sort_order: i64,
}

/// One physical stock deduction (kit-expanded leaf or direct line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafDeduction {
    /// Leaf physical product.
    pub product_id: ProductId,
    /// Handled unit (selling unit, or leaf base unit).
    pub unit_id: UnitId,
    /// Positive magnitude in unit micros.
    pub quantity_micro: i64,
    /// Allow the level to go negative (transfers always refuse; sales honor
    /// the actor's NegativeStockOverride grant).
    pub authorize_negative_stock: bool,
}

/// One tender line for persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutPayment {
    /// Tender method code (`CASH`, `CREDIT`, `CHEQUE`, `CARD`).
    pub method_code: String,
    /// Tendered amount in centimes.
    pub amount_centimes: i64,
}

/// Priced totals carried into the atomic write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutTotals {
    /// HT subtotal after all discounts.
    pub subtotal_ht: i64,
    /// Total TVA.
    pub total_tva: i64,
    /// Total global discount allocated.
    pub global_discount: i64,
    /// Timbre fiscal.
    pub timbre: i64,
    /// Grand total.
    pub total_ttc: i64,
    /// Total tendered.
    pub paid_total: i64,
    /// Change owed.
    pub change: i64,
}

/// Stored checkout outcome (serializable receipt for idempotent replay).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CheckoutReceipt {
    /// Stored document id (UUID string).
    pub document_id: String,
    /// Gapless human number.
    pub document_number: String,
    /// HT subtotal after all discounts.
    pub subtotal_ht_centimes: String,
    /// Total TVA.
    pub tva_total_centimes: String,
    /// Total global discount allocated.
    pub discount_total_centimes: String,
    /// Timbre fiscal.
    pub timbre_centimes: String,
    /// Grand total.
    pub total_ttc_centimes: String,
    /// Total tendered.
    pub paid_amount_centimes: String,
    /// Change owed.
    pub change_amount_centimes: String,
    /// PENDING outbox print job id.
    pub print_job_id: String,
}

/// Atomic sale engine port (single `run_transaction` inside).
pub trait CheckoutRepository: Send + Sync {
    /// Execute the full atomic checkout (idempotency, sequence, document,
    /// stock, ledger, print outbox, command log) and return the receipt.
    fn checkout_atomic(
        &self,
        command: &CheckoutCommand,
    ) -> Result<CheckoutReceipt, ApplicationError>;
}

/// Document persistence port (SQLite repository in infrastructure).
pub trait DocumentRepository: Send + Sync {
    /// Park an editable draft (no sequence number consumed).
    /// `origin_device_id` is the TerminalContext-resolved terminal identifier.
    fn create_draft(
        &self,
        document: &Document,
        items: &[DocumentItem],
        origin_device_id: &str,
    ) -> Result<(), ApplicationError>;

    /// Discard a parked draft (DRAFT -> CANCELLED; sealed states untouched).
    fn cancel_draft(&self, id: DocumentId) -> Result<(), ApplicationError>;

    /// Fetch a document header by id.
    fn get_by_id(&self, id: DocumentId) -> Result<Option<Document>, ApplicationError>;

    /// Fetch a document header by (type, fiscal year, number).
    fn get_by_number(
        &self,
        doc_type_code: &str,
        fiscal_year: i32,
        number: &str,
    ) -> Result<Option<Document>, ApplicationError>;

    /// Fetch all item snapshots of a document in display order.
    fn get_items(&self, id: DocumentId) -> Result<Vec<DocumentItem>, ApplicationError>;

    /// Fetch all payment rows of a document.
    fn get_payments(&self, id: DocumentId) -> Result<Vec<StoredPayment>, ApplicationError>;

    /// Latest print-job status for a document (`NONE` when absent).
    fn get_print_status(&self, id: DocumentId) -> Result<String, ApplicationError>;

    /// Insert a finalized document with items and payments (used by tests
    /// and repair flows; the sale engine uses `checkout_atomic` instead).
    /// `origin_device_id` is the TerminalContext-resolved terminal identifier.
    fn finalize_document(
        &self,
        document: &Document,
        items: &[DocumentItem],
        payments: &[StoredPayment],
        origin_device_id: &str,
    ) -> Result<(), ApplicationError>;

    /// Execute a single-shot DEVIS-to-BL conversion atomically (ADR-040):
    /// idempotency, source lock + validation, gapless BL number, byte-copy
    /// items, leaf stock deductions (SALE_OUT), source seal to CONVERTED,
    /// audit event, print outbox job, command log completion — all inside
    /// one `run_transaction`. Any failure rolls back everything.
    fn convert_devis_to_bl(
        &self,
        command: &ConvertDevisToBlCommand,
    ) -> Result<ConvertDevisToBlResult, ApplicationError>;

    /// Execute a single-shot BL-to-FACTURE financial closing atomically
    /// (ADR-040/041/046): idempotency, source lock + validation, exact
    /// tender settlement (cash movement and/or credit ledger), gapless
    /// FACTURE number, byte-copy items, source seal to CONVERTED, audit
    /// event, print outbox job, command log completion — all inside one
    /// `run_transaction`. Zero stock writes by invariant (ADR-046).
    fn convert_bl_to_facture(
        &self,
        command: &ConvertBlToFactureCommand,
    ) -> Result<ConvertBlToFactureResult, ApplicationError>;

    /// Search document headers (read-only, newest first).
    fn search_documents(
        &self,
        filter: &DocumentSearchFilter,
    ) -> Result<Vec<DocumentSummary>, ApplicationError>;
}

/// Read-only document search filter (all fields optional except limit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSearchFilter {
    /// Document kind code (`DEVIS`, `BL`, `FACTURE`, `TICKET`).
    pub doc_type: Option<String>,
    /// Lifecycle status code (`FINALIZED`, `CONVERTED`, `DRAFT`, …).
    pub status: Option<String>,
    /// Partial match against human number or customer name.
    pub query: Option<String>,
    /// Max rows, clamped to 1..=100 upstream (20 when unset).
    pub limit: Option<u32>,
}

/// Read-only document summary row for search results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSummary {
    /// Document id (UUID string).
    pub id: String,
    /// Gapless human number.
    pub number: String,
    /// Document kind code.
    pub doc_type: String,
    /// Lifecycle status code.
    pub status: String,
    /// Owning customer id, if any.
    pub party_id: Option<String>,
    /// Owning customer name, if any.
    pub party_name: Option<String>,
    /// Grand total in centimes.
    pub ttc_centimes: i64,
    /// Stored line count.
    pub items_count: i64,
    /// Creation instant (RFC3339).
    pub created_at: String,
}

/// Single-shot DEVIS-to-BL conversion command (all i64; pre-validated and
/// kit-expanded by the use case, mirroring `CheckoutCommand`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertDevisToBlCommand {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Canonical request hash input built by the use case.
    pub request_canonical: String,
    /// Source DEVIS document id.
    pub source_document_id: DocumentId,
    /// Terminal identifier resolved from TerminalContext (never client input).
    pub terminal_id: String,
    /// Leaf stock deductions (kit-expanded, base-unit micros).
    pub leaf_deductions: Vec<ConvertLeafDeduction>,
    /// Actor holds NegativeStockOverride (authoritative per-line handling in txn).
    pub authorize_negative_stock: bool,
    /// Acting operator.
    pub actor: UserId,
}

/// One expanded leaf deduction for a converted line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertLeafDeduction {
    /// Leaf physical product.
    pub product_id: ProductId,
    /// Leaf base unit.
    pub unit_id: UnitId,
    /// Positive magnitude in base micros.
    pub quantity_base_micro: i64,
}

/// Stored conversion outcome (serializable receipt for idempotent replay).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConvertDevisToBlResult {
    /// New BL document id (UUID string).
    pub new_document_id: String,
    /// New BL gapless human number.
    pub new_document_number: String,
    /// New BL status code (`FINALIZED`).
    pub new_document_status: String,
    /// Copied grand total in centimes.
    pub total_ttc_centimes: i64,
    /// Copied line count.
    pub items_count: u32,
    /// Per-line negative-stock override notices (empty when stock sufficed).
    pub warnings: Vec<String>,
}

/// Single-shot BL-to-FACTURE financial closing command (all i64).
///
/// Tender lines reuse [`CheckoutPayment`] verbatim (same method codes, same
/// positive-amount contract); settlement must equal the source total
/// exactly — this closing carries no change concept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertBlToFactureCommand {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Canonical request hash input built by the use case.
    pub request_canonical: String,
    /// Source BL document id.
    pub source_document_id: DocumentId,
    /// Terminal identifier resolved from TerminalContext (never client input).
    pub terminal_id: String,
    /// Customer-facing receipt language for the new FACTURE (never defaulted).
    pub document_language: DocumentLanguage,
    /// Tender lines covering the source total exactly.
    pub payments: Vec<CheckoutPayment>,
    /// Credit amount covered by CREDIT lines (0 when none).
    pub credit_amount_centimes: i64,
    /// Actor holds CreditLimitOverride (authoritative re-check in txn).
    pub credit_override_granted: bool,
    /// Acting operator.
    pub actor: UserId,
}

/// Stored financial-closing outcome (serializable for idempotent replay).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConvertBlToFactureResult {
    /// New FACTURE document id (UUID string).
    pub new_document_id: String,
    /// New FACTURE gapless human number.
    pub new_document_number: String,
    /// New FACTURE status code (`FINALIZED`).
    pub new_document_status: String,
    /// Settled grand total in centimes (source copy).
    pub total_ttc_centimes: i64,
    /// Copied line count.
    pub items_count: u32,
    /// Customer balance after settlement, when credit was used.
    pub customer_new_balance_centimes: Option<i64>,
}

/// Return processing persistence port (TASK-BRK-045).
///
/// Reads served by the existing `DocumentRepository` (`get_by_id`,
/// `get_items`); this port covers the return-specific cumulative query and
/// the single atomic write bundle. The SQLite adapter implements the
/// bundle in one `BEGIN IMMEDIATE` transaction (idempotency, AVOIR header +
/// linked items, `RETURN_IN` legs, cash/ledger legs, audit, command log).
pub trait ReturnRepository: Send + Sync {
    /// Already-returned micros per source item id (UUID string key).
    fn cumulative_returned_quantities(
        &self,
        source_document_id: DocumentId,
    ) -> Result<std::collections::HashMap<String, i64>, ApplicationError>;

    /// Persist one validated return atomically (or replay the stored
    /// receipt when `idempotency_key` was seen with an identical canonical
    /// request).
    fn persist_return(
        &self,
        order: &PersistReturnOrder,
    ) -> Result<PersistReturnReceipt, ApplicationError>;
}

/// One validated return line for atomic persistence (all i64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistReturnLine {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned product (leaf as displayed on the source line).
    pub product_id: ProductId,
    /// Selling unit handled.
    pub unit_id: UnitId,
    /// Returned magnitude in selling-unit micros.
    pub quantity_micro: i64,
    /// Returned magnitude in base micros.
    pub quantity_base_micro: i64,
    /// Agreed unit price in centimes (source copy).
    pub unit_price_centimes: i64,
    /// Pro-rated line discount in centimes.
    pub discount_centimes: i64,
    /// Computed line HT after discount.
    pub line_ht_centimes: i64,
    /// Computed line TVA.
    pub line_tva_centimes: i64,
    /// Computed line TTC.
    pub line_ttc_centimes: i64,
    /// Purchase price snapshot (margin base, source copy).
    pub cost_price_centimes: i64,
    /// Display order.
    pub sort_order: i64,
    /// Operator reason, if any.
    pub reason: Option<String>,
}

/// Validated return order for atomic persistence (all i64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistReturnOrder {
    /// Client idempotency key (UUID string).
    pub idempotency_key: String,
    /// Canonical request hash input built by the use case.
    pub request_canonical: String,
    /// Returned-against source document id.
    pub source_document_id: DocumentId,
    /// Terminal identifier resolved from TerminalContext (never client input).
    pub terminal_id: String,
    /// Selling warehouse (source copy).
    pub warehouse_id: WarehouseId,
    /// Customer-facing receipt language (source copy).
    pub document_language: DocumentLanguage,
    /// Computed totals.
    pub subtotal_ht_centimes: i64,
    /// Computed totals.
    pub vat_total_centimes: i64,
    /// Computed totals (timbre is never refunded: always 0).
    pub timbre_centimes: i64,
    /// Computed totals.
    pub ttc_centimes: i64,
    /// Cash refund leg (`REFUND_OUT` on the OPEN session), if any.
    pub cash_refund: Option<ReturnCashLeg>,
    /// Customer-ledger CREDIT leg (debt reduction), if any.
    pub credit_refund: Option<ReturnCreditLeg>,
    /// Validated return lines.
    pub lines: Vec<PersistReturnLine>,
    /// Acting operator.
    pub actor: UserId,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Cash refund leg of a return order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnCashLeg {
    /// OPEN cash session id disbursing the refund.
    pub cash_session_id: String,
    /// Refund magnitude in centimes (stored negated as `REFUND_OUT`).
    pub amount_centimes: i64,
}

/// Customer-ledger CREDIT leg of a return order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnCreditLeg {
    /// Credited customer id.
    pub customer_id: CustomerId,
    /// Refund magnitude in centimes (debt reduction).
    pub amount_centimes: i64,
}

/// Stored return outcome (serializable for idempotent replay).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PersistReturnReceipt {
    /// New AVOIR document id (UUID string).
    pub return_document_id: String,
    /// New AVOIR gapless human number.
    pub return_document_number: String,
}

/// Stored payment row view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPayment {
    /// Stored row id (TEXT UUID).
    pub id: String,
    /// Owning document.
    pub document_id: DocumentId,
    /// Customer for credit lines, if any.
    pub customer_id: Option<CustomerId>,
    /// Tendered amount in centimes.
    pub amount_centimes: i64,
    /// Tender method code.
    pub method_code: String,
    /// Cash session id, if any.
    pub cash_session_id: Option<String>,
}

/// Gapless sequence port (standalone allocation; the atomic checkout
/// allocates inside its own transaction for gapless safety).
pub trait DocumentSequenceRepository: Send + Sync {
    /// Allocate the next number for (type, year); returns the bare integer.
    fn allocate_next(&self, doc_type_code: &str, fiscal_year: i32)
        -> Result<i64, ApplicationError>;
}

/// Idempotency log port.
///
/// `command_log` is append-only (UPDATE/DELETE forbidden by trigger), so the
/// protocol is lookup-then-append: at most one completed row per command id.
/// A duplicate append means a lost race; the caller re-reads and replays.
pub trait CommandLogRepository: Send + Sync {
    /// Look up a stored command: id, request hash, response payload.
    fn lookup(&self, command_id: &str) -> Result<Option<StoredCommand>, ApplicationError>;

    /// Append one completed command row (fails on duplicate command id).
    fn append_completed(
        &self,
        command_id: &str,
        request_hash: &str,
        command_name: &str,
        response_payload: &str,
        actor: UserId,
    ) -> Result<(), ApplicationError>;
}

/// Stored idempotency row view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCommand {
    /// Client command id.
    pub command_id: String,
    /// SHA-256 of the canonical request.
    pub request_hash: String,
    /// Response payload JSON (empty while in flight).
    pub response_payload: String,
}

/// Customer credit port (SQLite repository in infrastructure).
///
/// The three TASK-BRK-006 methods (`get_credit_profile`,
/// `record_credit_debit`, `update_cached_balance`) are frozen and consumed
/// by the sale engine; the methods below extend the contract without
/// altering them.
pub trait CustomerRepository: Send + Sync {
    /// Credit profile for authorization decisions.
    fn get_credit_profile(
        &self,
        customer_id: CustomerId,
    ) -> Result<Option<CreditProfile>, ApplicationError>;

    /// Append a DEBIT ledger line; returns the stored entry id.
    fn record_credit_debit(
        &self,
        customer_id: CustomerId,
        amount_centimes: i64,
        running_balance_centimes: i64,
        document_id: DocumentId,
        actor: UserId,
    ) -> Result<String, ApplicationError>;

    /// Refresh the cached customer balance.
    fn update_cached_balance(
        &self,
        customer_id: CustomerId,
        new_balance_centimes: i64,
    ) -> Result<(), ApplicationError>;

    /// Persist a new customer; returns its id.
    /// `origin_device_id` is the TerminalContext-resolved terminal identifier.
    fn create_customer(
        &self,
        customer: &Customer,
        origin_device_id: &str,
    ) -> Result<CustomerId, ApplicationError>;

    /// Update mutable customer fields.
    fn update_customer(
        &self,
        customer_id: CustomerId,
        update: &CustomerUpdate,
    ) -> Result<(), ApplicationError>;

    /// Prefix/multilingual search capped at `limit` customers.
    fn search_customers(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CustomerSummary>, ApplicationError>;

    /// Full customer detail view.
    fn get_customer(&self, customer_id: CustomerId) -> Result<Option<Customer>, ApplicationError>;

    /// List customers ordered by name.
    fn list_customers(&self, active_only: bool) -> Result<Vec<CustomerSummary>, ApplicationError>;

    /// Append a CREDIT ledger line and decrement the balance atomically.
    /// ADR-027: the full amount is cash-collected into the active session.
    fn record_credit_payment(
        &self,
        customer_id: CustomerId,
        amount_centimes: i64,
        reference: Option<String>,
        allow_overpayment: bool,
        actor: UserId,
        terminal_id: &str,
    ) -> Result<PaymentRecord, ApplicationError>;

    /// Ledger rows in ledger order with server-side running balances.
    fn get_statement(
        &self,
        customer_id: CustomerId,
        from_date: Option<String>,
        to_date: Option<String>,
    ) -> Result<Vec<LedgerEntryRow>, ApplicationError>;
}

/// Customer creation payload (validated by the use case first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCustomer {
    /// Display name (trimmed, 2..=100 chars).
    pub name: String,
    /// Optional unique phone (Algerian format when present).
    pub phone: Option<String>,
    /// Optional address.
    pub address: Option<String>,
    /// Initial credit policy.
    pub credit_policy: CreditPolicy,
    /// Limit in centimes (required when policy is Limited).
    pub credit_limit_centimes: i64,
}

/// Sparse customer update (only `Some` fields are written).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomerUpdate {
    /// Replacement display name.
    pub name: Option<String>,
    /// Replacement phone (`Some(None)` clears it).
    pub phone: Option<Option<String>>,
    /// Replacement address (`Some(None)` clears it).
    pub address: Option<Option<String>>,
    /// Replacement credit policy.
    pub credit_policy: Option<CreditPolicy>,
    /// Replacement limit (required when policy becomes Limited).
    pub credit_limit_centimes: Option<i64>,
    /// Replacement active flag.
    pub is_active: Option<bool>,
}

/// Customer list row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerSummary {
    /// Stable identifier.
    pub id: CustomerId,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Policy code.
    pub credit_policy: CreditPolicy,
}

/// Recorded credit payment outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentRecord {
    /// Stored ledger entry id.
    pub ledger_entry_id: String,
    /// Balance after the payment, in centimes.
    pub new_balance_centimes: i64,
}

/// One statement line with its server-side running balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntryRow {
    /// `DEBIT` or `CREDIT`.
    pub entry_type: String,
    /// Entry amount in centimes.
    pub amount_centimes: i64,
    /// Referencing document kind, if any.
    pub reference_type: Option<String>,
    /// Referencing document id, if any.
    pub reference_id: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Running balance after this entry, in centimes.
    pub balance_after_centimes: i64,
}

/// Transactional outbox port for receipt print jobs.
/// Bounded retry policy: at most [`PRINT_MAX_ATTEMPTS`] physical delivery
/// attempts per job; the worker enforces the terminal `FAILED` transition.
pub trait PrintJobRepository: Send + Sync {
    /// Enqueue one PENDING job; returns its stored id.
    /// `document_id` is `None` for non-sale jobs (debt receipts, Z-reports,
    /// test prints) that reference no row in `documents`.
    fn enqueue_job(
        &self,
        document_id: Option<DocumentId>,
        job_type: &str,
        payload_json: &str,
    ) -> Result<String, ApplicationError>;

    /// Atomically claim one PENDING job (sets PROCESSING + claimed_at).
    fn claim_next_pending(&self) -> Result<Option<PrintJobRecord>, ApplicationError>;

    /// Mark a claimed job PROCESSING (explicit transition helper).
    fn mark_processing(&self, job_id: &str) -> Result<(), ApplicationError>;

    /// Mark a job DONE after a successful physical send.
    fn mark_done(&self, job_id: &str) -> Result<(), ApplicationError>;

    /// Mark a job FAILED after exhausted retries, recording the reason.
    fn mark_failed(&self, job_id: &str, reason: &str) -> Result<(), ApplicationError>;

    /// Release a job back to PENDING with attempt_count + 1 (retry path),
    /// recording the failure reason and tagging the row as a duplicata
    /// (`is_duplicate = 1`) so retried receipts render watermarked.
    fn release_for_retry(&self, job_id: &str, last_error: &str) -> Result<(), ApplicationError>;

    /// Release a job back to PENDING untouched (legacy transport-unavailable
    /// path; consumes no attempt budget). Retained for API compatibility;
    /// the worker no longer uses it for unavailable printers (that path now
    /// consumes bounded retry budget via `release_for_retry`).
    fn release_unclaimed(&self, job_id: &str) -> Result<(), ApplicationError>;

    /// Latest job state for a document, if any job exists.
    fn get_job_status(
        &self,
        document_id: DocumentId,
    ) -> Result<Option<PrintJobStatusRow>, ApplicationError>;

    /// Enqueue a DUPLICATA reprint row for a document; returns its id.
    /// Never touches the original job row (append-only history).
    fn enqueue_reprint(&self, document_id: DocumentId) -> Result<String, ApplicationError>;

    /// Reset PROCESSING jobs claimed longer than `timeout_secs` ago back to
    /// PENDING (crash recovery on worker startup). Returns rows reset.
    fn reclaim_stale_processing(&self, timeout_secs: u64) -> Result<u64, ApplicationError>;

    /// Reconcile abandoned PENDING jobs on startup: any row still PENDING
    /// with `attempt_count >= PRINT_MAX_ATTEMPTS` or `created_at` older than
    /// `stale_age_secs` transitions deterministically to FAILED with
    /// `last_error = 'ABANDONED_STALE_SESSION'`. Returns rows abandoned.
    fn reconcile_stale_pending(&self, stale_age_secs: u64) -> Result<u64, ApplicationError>;
}

/// Maximum physical delivery attempts per print job (bounded retry budget).
pub const PRINT_MAX_ATTEMPTS: i64 = 3;
/// Default staleness horizon for abandoned PENDING reconciliation (1 hour).
pub const PRINT_STALE_PENDING_SECS: u64 = 3600;

/// Claimed outbox job view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintJobRecord {
    /// Stored row id.
    pub job_id: String,
    /// Owning document id; `None` for non-sale jobs.
    pub document_id: Option<DocumentId>,
    /// Job kind code (`RECEIPT`, `DUPLICATA`, `DEBT_RECEIPT`).
    pub job_type: String,
    /// Physical send attempts so far.
    pub attempt_count: i64,
    /// Stored `payload_json` column (debt receipt document JSON for
    /// non-sale jobs; sales jobs carry their verbatim payload).
    pub payload_json: String,
}

/// Latest job state view for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintJobStatusRow {
    /// Stored row id.
    pub job_id: String,
    /// Status code (`PENDING`, `PROCESSING`, `DONE`, `FAILED`).
    pub status_code: String,
    /// Physical send attempts so far.
    pub attempt_count: i64,
}

/// Stored print-job identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrintJobId(pub String);

/// Serial printer configuration view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterSettings {
    /// COM port name (`""` when unconfigured).
    pub serial_port: String,
    /// Baud rate.
    pub serial_baud: u32,
    /// Paper width in dots.
    pub paper_width_dots: u32,
}

/// Printer configuration port (`settings` table in infrastructure).
pub trait PrinterSettingsPort: Send + Sync {
    /// Read the current serial printer configuration (with safe defaults).
    fn get_printer_settings(&self) -> Result<PrinterSettings, ApplicationError>;

    /// Persist the serial port and baud rate (ADMIN-gated upstream).
    fn update_printer_settings(
        &self,
        serial_port: &str,
        serial_baud: u32,
    ) -> Result<(), ApplicationError>;
}

/// Store identity + diagnostics port (`store.*` / `printing.*` keys in
/// the existing `settings` table — zero schema changes).
pub trait StoreIdentityPort: Send + Sync {
    /// Load the identity (absent keys read as empty strings).
    fn load_identity(&self) -> Result<baraka_domain::printing::StoreIdentity, ApplicationError>;

    /// Persist the identity (ADMIN-gated + audited upstream).
    fn save_identity(
        &self,
        identity: &baraka_domain::printing::StoreIdentity,
        actor: &str,
    ) -> Result<(), ApplicationError>;

    /// Device kind (`"physical"` default, `"capture"` emulator).
    fn device_kind(&self) -> Result<String, ApplicationError>;

    /// Persist the diagnostics device kind (ADMIN-gated + audited upstream).
    fn save_device(&self, kind: &str, actor: &str) -> Result<(), ApplicationError>;

    /// Append a chain-linked settings audit row (diagnostics + saves).
    fn append_settings_audit(
        &self,
        actor: &str,
        action: &str,
        detail: &str,
    ) -> Result<(), ApplicationError>;
}

/// Shadow-capture sink (immutable `.escpos` artifacts; test print only —
/// business print jobs are NEVER shadow-captured).
pub trait CaptureSink: Send + Sync {
    /// Write the exact byte stream; returns the artifact file name.
    fn write_capture(&self, bytes: &[u8]) -> Result<String, ApplicationError>;
}

/// Printer transport port (Serial QMessageConnection in infrastructure).
pub trait PrinterTransportPort: Send + Sync {
    /// Transmit ESC/POS raster bytes to the physical printer.
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError>;

    /// True when a configured transport endpoint is reachable.
    fn is_available(&self) -> bool;
}

/// Receipt raster renderer port (pure-Rust pipeline in infrastructure).
pub trait ReceiptRendererPort: Send + Sync {
    /// Render a receipt document into ESC/POS-ready raster command bytes.
    fn render(
        &self,
        receipt: &baraka_domain::printing::ReceiptDocument,
    ) -> Result<Vec<u8>, ApplicationError>;

    /// Shared layout blueprint (Amendment 1 preview source). Default:
    /// empty (render-only fakes keep compiling; the raster renderer
    /// overrides with the exact lines it rasterizes).
    fn preview_lines(
        &self,
        _receipt: &baraka_domain::printing::ReceiptDocument,
    ) -> Result<Vec<baraka_domain::printing::PreviewLine>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Render a debt-payment receipt (non-sale print job; no
    /// corresponding row in `documents`). Default: not implemented
    /// (test fakes that never exercise this path keep compiling).
    fn render_debt_receipt(
        &self,
        _receipt: &baraka_domain::printing::DebtReceiptDocument,
    ) -> Result<Vec<u8>, ApplicationError> {
        Err(ApplicationError::Port(
            "debt receipt rendering not implemented".to_string(),
        ))
    }
}

/// Cash session view (application-level projection of the domain aggregate).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CashSessionView {
    /// Stable identifier (TEXT UUID).
    pub id: String,
    /// Selling warehouse.
    pub warehouse_id: String,
    /// Terminal identifier (== device identifier).
    pub terminal_id: String,
    /// Status code (`OPEN`, `CLOSING`, `CLOSED`).
    pub status: String,
    /// Operator who opened the session.
    pub opened_by: String,
    /// Opening operator username.
    pub opened_by_username: String,
    /// Opening instant (RFC3339).
    pub opened_at: String,
    /// Opening float in centimes.
    pub opening_float_centimes: i64,
    /// Operator who closed, if closed.
    pub closed_by: Option<String>,
    /// Closing operator username, if closed.
    pub closed_by_username: Option<String>,
    /// Closing instant, if closed.
    pub closed_at: Option<String>,
    /// Physically counted amount, if supplied.
    pub actual_counted_centimes: Option<i64>,
    /// Frozen expected total (populated at BeginClosing).
    pub system_expected_centimes: Option<i64>,
    /// Counted minus expected (populated at ConfirmClosing).
    pub variance_centimes: Option<i64>,
    /// Optional closing notes.
    pub closing_notes: Option<String>,
    /// Optimistic concurrency revision.
    pub revision: i64,
    /// Row creation instant (RFC3339).
    pub created_at: String,
    /// Row update instant (RFC3339).
    pub updated_at: String,
}

/// Cash movement view.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CashMovementView {
    /// Stable identifier (TEXT UUID).
    pub id: String,
    /// Owning session.
    pub cash_session_id: String,
    /// Movement type code.
    pub movement_type: String,
    /// Signed amount in centimes.
    pub amount_centimes: i64,
    /// Referencing business kind, if any.
    pub reference_type: Option<String>,
    /// Referencing business id, if any.
    pub reference_id: Option<String>,
    /// Human description, if any.
    pub description: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator.
    pub created_by: String,
    /// Acting operator username.
    pub created_by_username: String,
}

/// Totals grouped by movement type for a session report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CashMovementTotal {
    /// Movement type code.
    pub movement_type: String,
    /// Signed total in centimes.
    pub total_centimes: i64,
    /// Row count for this type.
    pub count: i64,
}

/// Session report: header + ordered movements + grouped totals.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CashSessionReport {
    /// Session header.
    pub session: CashSessionView,
    /// Movements in creation order.
    pub movements: Vec<CashMovementView>,
    /// Totals grouped by movement type.
    pub totals_by_type: Vec<CashMovementTotal>,
    /// True when status == CLOSED (final Z-Report).
    pub is_final: bool,
}

/// Cash session persistence port (SQLite repository in infrastructure).
///
/// Every financial method is idempotent via `command_log`
/// Lookup-then-Append; reads carry no idempotency key.
pub trait CashSessionRepository: Send + Sync {
    /// Open a session with its OPENING_FLOAT movement atomically.
    fn open_session(
        &self,
        warehouse_id: WarehouseId,
        opening_float_centimes: i64,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError>;

    /// Transition OPEN -> CLOSING and freeze the expected total.
    fn begin_closing(
        &self,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError>;

    /// Transition CLOSING -> OPEN and clear the frozen expected value.
    fn cancel_closing(
        &self,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError>;

    /// Transition CLOSING -> CLOSED with variance computation.
    #[allow(clippy::too_many_arguments)]
    fn confirm_closing(
        &self,
        session_id: &str,
        actual_counted_centimes: i64,
        closing_notes: Option<String>,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError>;

    /// Active session for a terminal (OPEN or CLOSING), if any.
    fn get_active_session(
        &self,
        terminal_id: &str,
    ) -> Result<Option<CashSessionView>, ApplicationError>;

    /// Record a drawer expense (EXPENSE_OUT) on the OPEN session.
    fn record_expense(
        &self,
        amount_centimes: i64,
        description: &str,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashMovementView, ApplicationError>;

    /// Session report (interim X-Report or final Z-Report).
    fn get_session_report(&self, session_id: &str) -> Result<CashSessionReport, ApplicationError>;
}

// Supplier / purchase ports (TASK-BRK-011) --------------------------------
// Views use `i64` centimes internally; the IPC layer stringifies at the
// wire edge exactly like the cash views above.

/// Supplier persistence view (application-level projection).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupplierView {
    /// Stable identifier (TEXT UUID).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Address, if registered.
    pub address: Option<String>,
    /// NIF (tax registration), if registered.
    pub nif: Option<String>,
    /// RC (registre de commerce), if registered.
    pub rc: Option<String>,
    /// Notes, if any.
    pub notes: Option<String>,
    /// Active flag.
    pub is_active: bool,
    /// Cached balance in centimes (owed to supplier).
    pub current_balance_centimes: i64,
    /// Origin device (terminal identifier at creation).
    pub origin_device_id: String,
    /// Row creation instant (RFC3339).
    pub created_at: String,
    /// Row update instant (RFC3339).
    pub updated_at: String,
}

/// Supplier creation payload (validated by the use case first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSupplier {
    /// Display name (trimmed, 2..=100 chars).
    pub name: String,
    /// Optional phone.
    pub phone: Option<String>,
    /// Optional address.
    pub address: Option<String>,
    /// Optional NIF.
    pub nif: Option<String>,
    /// Optional RC.
    pub rc: Option<String>,
    /// Optional notes.
    pub notes: Option<String>,
}

/// Sparse supplier update (only `Some` fields are written).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SupplierUpdate {
    /// Replacement display name.
    pub name: Option<String>,
    /// Replacement phone (`Some(None)` clears it).
    pub phone: Option<Option<String>>,
    /// Replacement address (`Some(None)` clears it).
    pub address: Option<Option<String>>,
    /// Replacement NIF (`Some(None)` clears it).
    pub nif: Option<Option<String>>,
    /// Replacement RC (`Some(None)` clears it).
    pub rc: Option<Option<String>>,
    /// Replacement notes (`Some(None)` clears it).
    pub notes: Option<Option<String>>,
    /// Replacement active flag.
    pub is_active: Option<bool>,
}

/// One supplier statement row with server-side running balance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupplierLedgerRow {
    /// Stored entry id.
    pub id: String,
    /// `DEBIT` or `CREDIT`.
    pub entry_type: String,
    /// Entry amount in centimes (positive magnitude).
    pub amount_centimes: i64,
    /// `PURCHASE_INVOICE` or `PAYMENT`.
    pub reference_type: String,
    /// Producing document/entry id.
    pub reference_id: String,
    /// Optional description.
    pub description: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator id.
    pub created_by: String,
    /// Running balance after this entry, in centimes.
    pub balance_after_centimes: i64,
}

/// Supplier statement: ledger rows plus the cached balance mirror.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplierStatement {
    /// Rows in ledger order.
    pub entries: Vec<SupplierLedgerRow>,
    /// Cached supplier balance for cross-verification.
    pub current_balance_centimes: i64,
}

/// Supplier master-data + statement port (SQLite repository).
pub trait SupplierRepository: Send + Sync {
    /// Persist a supplier; origin_device_id comes from TerminalContext.
    fn create_supplier(
        &self,
        supplier: &NewSupplier,
        origin_device_id: &str,
    ) -> Result<SupplierView, ApplicationError>;

    /// Sparse update of mutable supplier fields.
    fn update_supplier(
        &self,
        supplier_id: &str,
        update: &SupplierUpdate,
    ) -> Result<SupplierView, ApplicationError>;

    /// Full supplier view by id.
    fn get_supplier(&self, supplier_id: &str) -> Result<Option<SupplierView>, ApplicationError>;

    /// Prefix/multilingual search capped at `limit` suppliers.
    fn search_suppliers(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SupplierView>, ApplicationError>;

    /// Ledger rows in ledger order with running balances.
    fn get_statement(
        &self,
        supplier_id: &str,
    ) -> Result<Option<SupplierStatement>, ApplicationError>;
}

/// One purchase document line (input to create/receive/invoice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseLineInput {
    /// Product purchased.
    pub product_id: ProductId,
    /// Handling unit purchased in.
    pub unit_id: UnitId,
    /// Quantity in unit-micros (must be > 0).
    pub quantity_micro: i64,
    /// Estimated or actual unit cost in centimes (>= 0).
    pub unit_cost_centimes: i64,
}

/// Stored purchase line view.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PurchaseLineView {
    /// Stored document_items id.
    pub id: String,
    /// Product purchased.
    pub product_id: String,
    /// Handling unit.
    pub unit_id: String,
    /// Quantity in unit-micros.
    pub quantity_micro: i64,
    /// Quantity converted to base-unit micros.
    pub quantity_base_micro: i64,
    /// Unit cost in centimes.
    pub unit_cost_centimes: i64,
    /// Line total in centimes.
    pub line_total_centimes: i64,
}

/// Purchase document view (BC / BR / FF header + lines).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PurchaseDocumentView {
    /// Document id (TEXT UUID).
    pub id: String,
    /// Gapless human number.
    pub number: String,
    /// Document type code (`BC`, `BR`, `FF`).
    pub doc_type: String,
    /// Status code.
    pub status: String,
    /// Supplier id (party).
    pub supplier_id: String,
    /// Warehouse.
    pub warehouse_id: String,
    /// Predecessor document id (BR -> BC, FF -> BR), if any.
    pub source_document_id: Option<String>,
    /// Document total in centimes.
    pub total_centimes: i64,
    /// Priced lines.
    pub lines: Vec<PurchaseLineView>,
    /// Creation instant (RFC3339).
    pub created_at: String,
    /// Creator operator id.
    pub created_by: String,
}

/// Purchase engine port: BC/BR/FF documents, ledger, payments (SQLite).
///
/// Every mutating method is idempotent via `command_log`
/// Lookup-then-Append; `get_document` is a plain read.
pub trait PurchaseRepository: Send + Sync {
    /// Create a DRAFT purchase order (BC): zero stock/ledger effect.
    #[allow(clippy::too_many_arguments)]
    fn create_order(
        &self,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: &[PurchaseLineInput],
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError>;

    /// Replace the line set of a DRAFT purchase order.
    fn update_order(
        &self,
        document_id: &str,
        lines: &[PurchaseLineInput],
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError>;

    /// Receive goods: BR commit + PURCHASE_IN movements + BC rollup.
    /// `override_allowed` permits exceeding ordered quantities (audited).
    #[allow(clippy::too_many_arguments)]
    fn receive_goods(
        &self,
        source_bc_id: Option<&str>,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: &[PurchaseLineInput],
        override_allowed: bool,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError>;

    /// Record supplier invoice (FF): exactly one ledger DEBIT; stock only
    /// when no source BR (standalone combined receipt, ADR-029).
    #[allow(clippy::too_many_arguments)]
    fn record_invoice(
        &self,
        source_br_id: Option<&str>,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: &[PurchaseLineInput],
        invoice_total_centimes: i64,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError>;

    /// Record a supplier payment (ledger CREDIT + balance debit; cash
    /// SUPPLIER_PAYMENT_OUT only for CASH per ADR-030).
    #[allow(clippy::too_many_arguments)]
    fn record_payment(
        &self,
        supplier_id: &str,
        amount_centimes: i64,
        payment_method: &str,
        description: Option<String>,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<SupplierPaymentOutcome, ApplicationError>;

    /// Load any purchase document by id.
    fn get_document(
        &self,
        document_id: &str,
    ) -> Result<Option<PurchaseDocumentView>, ApplicationError>;
}

/// One daily sales aggregate row (all money `i64` centimes internally).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DailySalesRow {
    /// Calendar day (`YYYY-MM-DD`).
    pub day: String,
    /// Grand total in centimes.
    pub total_ttc_centimes: i64,
    /// HT subtotal in centimes.
    pub total_ht_centimes: i64,
    /// TVA total in centimes.
    pub total_tva_centimes: i64,
    /// Committed document count.
    pub document_count: i64,
    /// Mean basket in centimes (total_ttc / document_count, half away).
    pub average_basket_centimes: i64,
}

/// One per-product sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SalesByProductRow {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// French display name.
    pub name_fr: String,
    /// Quantity sold in base micros.
    pub quantity_sold_base_micro: i64,
    /// Revenue in centimes (line TTC sum).
    pub revenue_ttc_centimes: i64,
    /// Contributing line count.
    pub line_count: i64,
}

/// One per-category sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SalesByCategoryRow {
    /// Category id (`None` groups uncategorized products).
    pub category_id: Option<String>,
    /// Category code (`None` for uncategorized).
    pub category_code: Option<String>,
    /// French display name (`None` for uncategorized).
    pub name_fr: Option<String>,
    /// Revenue in centimes (line TTC sum).
    pub revenue_ttc_centimes: i64,
    /// Contributing line count.
    pub line_count: i64,
}

/// One per-product gross-margin row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GrossMarginRow {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// Revenue HT in centimes.
    pub revenue_ht_centimes: i64,
    /// Cost of goods sold in centimes (cost_price * selling qty; lines
    /// with a zero unit cost contribute 0).
    pub cost_total_centimes: i64,
    /// Margin in centimes (revenue_ht - cost_total).
    pub margin_centimes: i64,
    /// True when any contributing line had a zero unit cost, i.e. the
    /// margin is computed against untracked cost (TASK-BRK-012A).
    pub has_untracked_cost: bool,
}

/// One low-stock row (ADR-037: stockout / depletion signal).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LowStockRow {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// French display name.
    pub name_fr: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Warehouse code.
    pub warehouse_code: String,
    /// On-hand quantity in base micros (<= 0 by construction).
    pub quantity_base_micro: i64,
}

/// One credit-outstanding row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreditOutstandingRow {
    /// Customer id.
    pub customer_id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Policy code.
    pub credit_policy: String,
    /// Limit in centimes (`None` for UNLIMITED).
    pub credit_limit_centimes: Option<i64>,
    /// Current balance in centimes (> 0).
    pub current_balance_centimes: i64,
}

/// One stock-movement history row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockMovementHistoryRow {
    /// Movement id.
    pub id: String,
    /// Movement type code.
    pub movement_type: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Product id.
    pub product_id: String,
    /// Quantity in base micros (signed delta).
    pub quantity_base_micro: i64,
    /// Reference document type, if any.
    pub reference_type: Option<String>,
    /// Reference document id, if any.
    pub reference_id: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator id.
    pub created_by: String,
}

/// One cash-session history row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CashSessionHistoryRow {
    /// Session id.
    pub id: String,
    /// Terminal id.
    pub terminal_id: String,
    /// Status code.
    pub status: String,
    /// Opening instant (RFC3339).
    pub opened_at: String,
    /// Closing instant, if closed.
    pub closed_at: Option<String>,
    /// Opening float in centimes.
    pub opening_float_centimes: i64,
    /// Frozen expected total, if computed.
    pub system_expected_centimes: Option<i64>,
    /// Counted amount, if supplied.
    pub actual_counted_centimes: Option<i64>,
    /// Variance, if computed.
    pub variance_centimes: Option<i64>,
}

/// One user-activity (audit) row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserActivityRow {
    /// Monotonic sequence number.
    pub sequence_no: i64,
    /// Capture instant (RFC3339).
    pub timestamp: String,
    /// Acting operator id.
    pub user_id: String,
    /// Action code.
    pub action: String,
    /// Entity type.
    pub entity_type: String,
    /// Entity id.
    pub entity_id: String,
    /// Raw previous-value JSON, if any.
    pub old_value_json: Option<String>,
    /// Raw new-value JSON, if any.
    pub new_value_json: Option<String>,
}

/// Read-only reporting port (TASK-BRK-012).
///
/// Every method is a pure SELECT over transactional tables: no writes, no
/// idempotency keys, no materialized aggregates. RBAC (`ReportsView`) is
/// enforced by the use-case layer, never here.
pub trait ReportingRepository: Send + Sync {
    /// Daily aggregates for committed TICKET/FACTURE/BL documents in range.
    fn daily_sales(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<DailySalesRow>, ApplicationError>;

    /// Per-product aggregates for committed sale lines in range.
    fn sales_by_product(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<SalesByProductRow>, ApplicationError>;

    /// Per-category aggregates for committed sale lines in range.
    fn sales_by_category(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<SalesByCategoryRow>, ApplicationError>;

    /// Per-product gross margin for committed sale lines in range.
    /// cost_price_centimes is NOT NULL on every committed row, so every
    /// line contributes to its product bucket (no exclusions).
    fn gross_margin(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<GrossMarginRow>, ApplicationError>;

    /// All non-DISABLED customers with a positive balance, by balance desc.
    fn credit_outstanding(&self) -> Result<Vec<CreditOutstandingRow>, ApplicationError>;

    /// Filtered, paginated stock-movement history (newest first).
    #[allow(clippy::too_many_arguments)]
    fn stock_movement_history(
        &self,
        warehouse_id: Option<&str>,
        product_id: Option<&str>,
        movement_type: Option<&str>,
        date_from: &str,
        date_to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StockMovementHistoryRow>, ApplicationError>;

    /// Cash sessions in range (newest first).
    fn cash_sessions_history(
        &self,
        terminal_id: Option<&str>,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<CashSessionHistoryRow>, ApplicationError>;

    /// Products at or below zero on hand (ADR-037), most depleted first.
    /// `warehouse_id` narrows to one warehouse; `None` covers all.
    fn low_stock(&self, warehouse_id: Option<&str>) -> Result<Vec<LowStockRow>, ApplicationError>;

    /// Filtered, paginated audit-log rows (newest first, generic payload).
    fn user_activity(
        &self,
        user_id: Option<&str>,
        action: Option<&str>,
        date_from: &str,
        date_to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<UserActivityRow>, ApplicationError>;
}

/// Backup snapshot verification outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupVerification {
    /// Every `PRAGMA quick_check` row read exactly `"ok"`.
    pub quick_check_ok: bool,
    /// Rows returned by `PRAGMA foreign_key_check` (must be 0).
    pub foreign_key_violations: u64,
}

/// One `backups_log` row (application-level projection).
///
/// The fixed 0001 schema carries no file-path/size columns, so `backup_id`
/// stores the snapshot file name and callers rejoin it with the backup
/// directory to locate the file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupLogEntry {
    /// Row id (UUID string, also the metadata id).
    pub id: String,
    /// Snapshot file name (also the UNIQUE `backup_id`).
    pub backup_id: String,
    /// Snapshot file name (mirrors `backup_id`).
    pub file_name: String,
    /// Application version that wrote the snapshot.
    pub app_version: String,
    /// Embedded migration count in force at capture.
    pub schema_version: i64,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Origin terminal identifier.
    pub source_device_id: String,
    /// SHA-256 hex over the exact snapshot bytes.
    pub database_hash: String,
    /// Cipher profile (`NONE` on the unencrypted baseline).
    pub cipher_version: String,
    /// Lifecycle state (`SUCCESS`, `FAILED`, `CORRUPTED`, `PRUNED`).
    pub status: String,
}

/// Backup persistence port (TASK-BRK-051, ADR-010).
///
/// Snapshot creation uses SQLite's native online backup API (WAL-safe);
/// verification reopens the target read-only. File deletion is
/// best-effort rotation support and never touches the log.
///
/// Restore (TASK-BRK-054, ADR-062) extends the same port: `precheck_restore`
/// is strictly read-only, and `restore_backup` re-runs every verification
/// internally before quarantining the active database. (The brief names this
/// surface `BackupManagerPort`; the trait already exists here as
/// `BackupRepository`, so the methods land on it instead of a rename.)
pub trait BackupRepository: Send + Sync {
    /// Write a consistent snapshot of the live database to `dest`.
    fn snapshot_to(&self, dest: &std::path::Path) -> Result<(), ApplicationError>;

    /// Reopen `path` read-only and run integrity pragmas.
    fn verify_snapshot(
        &self,
        path: &std::path::Path,
    ) -> Result<BackupVerification, ApplicationError>;

    /// SHA-256 hex over the exact file bytes (streamed, bounded RAM).
    fn sha256_of(&self, path: &std::path::Path) -> Result<String, ApplicationError>;

    /// File size in bytes.
    fn file_size_of(&self, path: &std::path::Path) -> Result<u64, ApplicationError>;

    /// Append one `backups_log` row.
    fn record_backup(&self, entry: &BackupLogEntry) -> Result<(), ApplicationError>;

    /// Look up one `backups_log` row by backup id.
    fn get_backup(&self, backup_id: &str) -> Result<Option<BackupLogEntry>, ApplicationError>;

    /// Recent rows, newest first, capped at `limit`.
    fn list_backups(&self, limit: usize) -> Result<Vec<BackupLogEntry>, ApplicationError>;

    /// Rewrite a row's lifecycle state.
    fn mark_status(&self, backup_id: &str, status: &str) -> Result<(), ApplicationError>;

    /// Remove one snapshot file from disk (rotation support).
    fn delete_file(&self, path: &std::path::Path) -> Result<(), ApplicationError>;

    /// Embedded migration count in force.
    fn schema_version(&self) -> i64;

    /// Read-only restore precheck: the full verification report for one
    /// logged backup. Performs zero writes to any file or database.
    fn precheck_restore(
        &self,
        backup_id: &str,
        dest_dir: &std::path::Path,
    ) -> Result<baraka_domain::backup::PreRestoreReport, baraka_domain::backup::RestoreError>;

    /// Verified destructive restore: re-runs every verification internally,
    /// quarantines the active database, activates the backup copy, migrates
    /// it when older, and appends a chain-linked `RESTORE_PERFORMED` audit
    /// event. On post-quarantine failure the previous database is moved back.
    fn restore_backup(
        &self,
        backup_id: &str,
        dest_dir: &std::path::Path,
        actor: &str,
    ) -> Result<baraka_domain::backup::RestoreOutcome, baraka_domain::backup::RestoreError>;
}

/// Recorded supplier-payment outcome.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupplierPaymentOutcome {
    /// Stored supplier_ledger entry id.
    pub ledger_entry_id: String,
    /// Supplier cached balance after the payment.
    pub new_balance_centimes: i64,
    /// Cash movement id (CASH payments only).
    pub cash_movement_id: Option<String>,
}
