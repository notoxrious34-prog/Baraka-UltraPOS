//! Data-transfer objects exchanged with the React frontend.
//!
//! All money crosses the bridge as integer centimes (`i64`).

use baraka_domain::models::HealthSnapshot;
use serde::{Deserialize, Serialize};

/// Command failure serialized to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandErrorDto {
    /// Stable machine-readable code, e.g. `VALIDATION`.
    pub code: String,
    /// French user-facing message.
    pub message: String,
}

impl CommandErrorDto {
    /// Build a DTO from a code and a French message.
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

/// Health response served to the cockpit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthDto {
    /// Workspace version.
    pub version: String,
    /// RFC3339 capture time.
    pub captured_at: String,
    /// Offline-first readiness flag.
    pub offline_ready: bool,
}

impl From<HealthSnapshot> for HealthDto {
    fn from(snapshot: HealthSnapshot) -> Self {
        Self {
            version: snapshot.version,
            captured_at: snapshot.captured_at.to_rfc3339(),
            offline_ready: snapshot.offline_ready,
        }
    }
}

/// Username + password login request.
///
/// Never `Debug`-printable with the secret: the manual `Debug` impl below
/// redacts the password field.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequestDto {
    /// Raw login name as typed by the operator.
    pub username: String,
    /// Plaintext password (memory only, never logged or persisted).
    pub password: String,
}

impl std::fmt::Debug for LoginRequestDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginRequestDto")
            .field("username", &self.username)
            .field("password", &"***")
            .finish()
    }
}

/// Successful authentication response (all plain strings per IPC contract).
///
/// The manual `Debug` impl redacts the session token.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginResponseDto {
    /// Opaque volatile session token.
    pub session_token: String,
    /// Authenticated user id (UUID hyphenated string).
    pub user_id: String,
    /// Login name that authenticated.
    pub username: String,
    /// Role code (`VENDEUR`, `CAISSIER`, `GERANT`, `ADMIN`).
    pub role: String,
    /// Fixed session expiry (RFC3339).
    pub expires_at: String,
}

impl std::fmt::Debug for LoginResponseDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginResponseDto")
            .field("session_token", &"***")
            .field("user_id", &self.user_id)
            .field("username", &self.username)
            .field("role", &self.role)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Session revocation request.
///
/// The manual `Debug` impl redacts the session token.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutRequestDto {
    /// Token to revoke (unknown tokens are a silent no-op).
    pub session_token: String,
}

impl std::fmt::Debug for LogoutRequestDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogoutRequestDto")
            .field("session_token", &"***")
            .finish()
    }
}

// First-run bootstrap wire types (TASK-BRK-016, ADR-048) -------------------
// Unauthenticated by definition: no session token travels here. The Admin
// sets their own credentials; no default password exists anywhere.

/// First-run probe response: true only while the `users` table is empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFirstRunStatusResponse {
    /// True when the setup screen must be shown instead of the login form.
    pub requires_setup: bool,
}

/// First-admin creation request.
///
/// Never `Debug`-printable with the secret: the manual `Debug` impl below
/// redacts the password field, mirroring `LoginRequestDto`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapFirstAdminRequest {
    /// Client idempotency key (UUID v4 string; informational for this flow).
    pub command_id: String,
    /// Requested Admin login name.
    pub username: String,
    /// Requested Admin password (memory only, never logged or persisted).
    pub password: String,
}

impl std::fmt::Debug for BootstrapFirstAdminRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BootstrapFirstAdminRequest")
            .field("command_id", &self.command_id)
            .field("username", &self.username)
            .field("password", &"***")
            .finish()
    }
}

/// First-admin creation response (no session token: the frontend logs in
/// with the freshly created credentials through the standard login flow).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapFirstAdminResponse {
    /// Fresh operator id of the created Admin (UUID hyphenated string).
    pub user_id: String,
    /// Stored login name (trimmed).
    pub username: String,
}

// Catalog wire types ----------------------------------------------------
// Money travels as integer-centime strings, quantities/factors as integer
// micro strings. No float ever crosses this boundary.

/// One unit supplied at product creation (ids allocated server-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewProductUnitDto {
    /// Arabic unit name.
    pub unit_name_ar: String,
    /// French unit name.
    pub unit_name_fr: String,
    /// Base micros per selling unit (`"10000"` for the base unit).
    pub factor_micro: String,
    /// Optional barcode resolving to this unit.
    pub barcode: Option<String>,
    /// Sell price in centimes, e.g. `"15000"` for 150.00 DZD.
    pub sale_price_centimes: String,
    /// Purchase price in centimes.
    pub purchase_price_centimes: String,
    /// True for the single base unit.
    pub is_base_unit: bool,
    /// Default POS unit.
    pub is_default_selling: bool,
}

/// Product creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProductDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Unique business reference.
    pub sku: String,
    /// Optional product-level barcode.
    pub barcode: Option<String>,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category id (UUID string).
    pub category_id: Option<String>,
    /// True when the product is a kit header.
    pub is_kit: bool,
    /// Full unit set (exactly one base unit).
    pub units: Vec<NewProductUnitDto>,
}

/// Stored unit view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductUnitDto {
    /// Unit id (UUID string).
    pub unit_id: String,
    /// Owning product id (UUID string).
    pub product_id: String,
    /// Arabic unit name.
    pub unit_name_ar: String,
    /// French unit name.
    pub unit_name_fr: String,
    /// Base micros per selling unit.
    pub factor_micro: String,
    /// Barcode resolving to this unit, if any.
    pub barcode: Option<String>,
    /// Sell price in centimes.
    pub sale_price_centimes: String,
    /// Purchase price in centimes.
    pub purchase_price_centimes: String,
    /// True for the single base unit.
    pub is_base_unit: bool,
    /// Default POS unit.
    pub is_default_selling: bool,
}

/// Stored BOM line view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitItemDto {
    /// Component product id (UUID string).
    pub component_id: String,
    /// Component unit id (UUID string).
    pub unit_id: String,
    /// Quantity per kit, in unit micros.
    pub quantity_micro: String,
}

/// Full product detail view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductResponseDto {
    /// Product id (UUID string).
    pub product_id: String,
    /// Unique business reference.
    pub sku: String,
    /// Product-level barcode, if any.
    pub barcode: Option<String>,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category id, if any.
    pub category_id: Option<String>,
    /// Kit header flag.
    pub is_kit: bool,
    /// Active flag.
    pub is_active: bool,
    /// All associated units.
    pub units: Vec<ProductUnitDto>,
    /// BOM lines (empty for non-kits and list views).
    pub kit_items: Vec<KitItemDto>,
}

/// Single-product fetch request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetProductDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
}

/// One BOM line supplied at kit definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefineKitItemDto {
    /// Component product id (UUID string).
    pub component_id: String,
    /// Component unit id (UUID string).
    pub unit_id: String,
    /// Quantity per kit, in unit micros.
    pub quantity_micro: String,
}

/// Kit definition request (replaces the kit's full BOM).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefineKitDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Kit header id (UUID string).
    pub kit_id: String,
    /// Replacement BOM lines (must be non-empty).
    pub items: Vec<DefineKitItemDto>,
}

/// One unit upsert supplied at product update (`None` id allocates new).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateProductUnitDto {
    /// Existing unit id (UUID string), or `None` for a brand-new unit.
    pub unit_id: Option<String>,
    /// Arabic unit name.
    pub unit_name_ar: String,
    /// French unit name.
    pub unit_name_fr: String,
    /// Base micros per selling unit (`"10000"` for the base unit).
    pub factor_micro: String,
    /// Optional barcode resolving to this unit.
    pub barcode: Option<String>,
    /// Sell price in centimes, e.g. `"15000"` for 150.00 DZD.
    pub sale_price_centimes: String,
    /// Purchase price in centimes.
    pub purchase_price_centimes: String,
    /// True for the single base unit.
    pub is_base_unit: bool,
    /// Default POS unit.
    pub is_default_selling: bool,
}

/// Product update request (full-state payload; SKU stays immutable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateProductDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category id (UUID string), or `None` to clear it.
    pub category_id: Option<String>,
    /// Active flag.
    pub is_active: bool,
    /// Merged unit set after applying upserts.
    pub units: Vec<UpdateProductUnitDto>,
}

/// Catalog search request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogSearchRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Prefix/multilingual query (blank returns no rows).
    pub query: String,
    /// Max products (clamped to 1..=100 server-side).
    pub limit: Option<u32>,
}

/// Stored category view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryResponseDto {
    /// Category id (UUID string).
    pub id: String,
    /// Unique business code, e.g. `CAT-PLOMB`.
    pub code: String,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category id (UUID string), if nested.
    pub parent_id: Option<String>,
    /// Inactive categories accept no new products.
    pub is_active: bool,
}

/// Category creation request (active by default; code unique).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCategoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Unique business code, e.g. `CAT-PLOMB`.
    pub code: String,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category id (UUID string), if nested.
    pub parent_id: Option<String>,
}

/// Category update request (code stays immutable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCategoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Category id (UUID string).
    pub category_id: String,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category id (UUID string), or `None` to detach it.
    pub parent_id: Option<String>,
    /// Active flag.
    pub is_active: bool,
}

// Inventory wire types --------------------------------------------------
// Quantities travel as integer-micro strings; ids are UUID strings.

/// Warehouse creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWarehouseRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Unique business code (1..=20 chars).
    pub code: String,
    /// Display name (1..=100 chars).
    pub name: String,
}

/// Stored warehouse view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarehouseDto {
    /// Warehouse id (UUID string).
    pub id: String,
    /// Unique business code.
    pub code: String,
    /// Display name.
    pub name: String,
    /// Active flag.
    pub is_active: bool,
}

/// Warehouse list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListWarehousesResponseDto {
    /// Active warehouses ordered by code.
    pub warehouses: Vec<WarehouseDto>,
}

/// Stock level read request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStockLevelRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
    /// Warehouse id (UUID string).
    pub warehouse_id: String,
}

/// Stock level view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStockLevelResponseDto {
    /// Product id (UUID string).
    pub product_id: String,
    /// Warehouse id (UUID string).
    pub warehouse_id: String,
    /// Base unit id (UUID string; empty when unresolvable).
    pub base_unit_id: String,
    /// On-hand quantity in base micros.
    pub quantity_base_micro: String,
}

/// Manual stock correction request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjustStockRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Physical product id (UUID string).
    pub product_id: String,
    /// Target warehouse id (UUID string).
    pub warehouse_id: String,
    /// Handled selling unit id (UUID string).
    pub unit_id: String,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: String,
    /// `"INCREASE"` or `"DECREASE"` (explicit direction).
    pub direction: String,
    /// Mandatory human reason (min 3 chars after trim).
    pub reason: String,
    /// Request to allow a negative resulting level.
    pub authorize_negative_stock: bool,
}

/// Adjustment outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjustStockResponseDto {
    /// Stored movement id.
    pub movement_id: String,
    /// Level after the movement, in base micros.
    pub new_quantity_base_micro: String,
}

/// Warehouse transfer request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferStockRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Source warehouse id (UUID string).
    pub source_warehouse_id: String,
    /// Destination warehouse id (UUID string).
    pub destination_warehouse_id: String,
    /// Physical product id (UUID string).
    pub product_id: String,
    /// Handled selling unit id (UUID string).
    pub unit_id: String,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: String,
    /// Optional human reason.
    pub reason: Option<String>,
}

/// Transfer outcome (one OUT + one IN sharing a batch id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferStockResponseDto {
    /// Shared batch id.
    pub transfer_batch_id: String,
    /// Stored OUT movement id.
    pub out_movement_id: String,
    /// Stored IN movement id.
    pub in_movement_id: String,
    /// Source level after the movement, in base micros.
    pub source_new_quantity_base_micro: String,
    /// Destination level after the movement, in base micros.
    pub destination_new_quantity_base_micro: String,
}

// Sale wire types -------------------------------------------------------
// Money travels as integer-centime strings, quantities as integer-micro
// strings, ids as UUID strings. No float ever crosses this boundary.

/// One cart line for checkout or draft parking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartLineItemDto {
    /// Product id (UUID string).
    pub product_id: String,
    /// Selling unit id (UUID string).
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Explicit operator price (`Some` = override attempt).
    pub unit_price_centimes: Option<String>,
    /// Line discount in centimes.
    pub line_discount_centimes: String,
}

/// One tender line for checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLineDto {
    /// Tender method (`CASH`, `CREDIT`, `CHEQUE`, `CARD`).
    pub payment_method: String,
    /// Tendered amount in centimes.
    pub amount_centimes: String,
}

/// Atomic checkout request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutSaleRequest {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Customer id for credit sales (UUID string).
    pub customer_id: Option<String>,
    /// Document kind (`TICKET`, `FACTURE`, `BL`).
    pub document_type: String,
    /// Sale-time receipt language (`AR` or `FR`, mandatory, no default).
    pub document_language: String,
    /// Cart lines.
    pub lines: Vec<CartLineItemDto>,
    /// Global discount in centimes.
    pub global_discount_centimes: String,
    /// Tender lines.
    pub payments: Vec<PaymentLineDto>,
    /// Explicit in-session sale-on-shortage confirmation (TASK-BRK-033).
    /// Privileged roles only; ignored otherwise. Defaults to false.
    #[serde(default)]
    pub allow_negative_stock: bool,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Atomic checkout response (stored receipt shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutSaleResponse {
    /// Stored document id.
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

/// Read-only totals preview request (same cart shape as checkout).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewTotalsRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id (UUID string; shortage warnings read its levels).
    pub warehouse_id: String,
    /// Document kind (`TICKET`, `FACTURE`, `BL`).
    pub document_type: String,
    /// Cart lines.
    pub lines: Vec<CartLineItemDto>,
    /// Global discount in centimes.
    pub global_discount_centimes: String,
    /// Tender lines exactly as currently entered by the operator.
    pub payments: Vec<PaymentLineDto>,
}

/// One leaf-level stock shortage (TASK-BRK-033).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockShortageWarningDto {
    /// Short physical product id (UUID string).
    pub product_id: String,
    /// Commercial designation (FR) for the confirmation banner.
    pub product_name: String,
    /// Stock-keeping reference.
    pub sku: String,
    /// On-hand base micros at preview time (advisory).
    pub available_qty_micro: String,
    /// Requested base micros aggregated over the cart.
    pub requested_qty_micro: String,
}

/// Read-only totals preview response (commit-identical, all centime strings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewTotalsResponse {
    /// HT subtotal after all discounts.
    pub subtotal_ht_centimes: String,
    /// Total TVA.
    pub tva_total_centimes: String,
    /// Total global discount allocated.
    pub global_discount_centimes: String,
    /// Timbre fiscal.
    pub timbre_centimes: String,
    /// Grand total.
    pub total_ttc_centimes: String,
    /// Total tendered.
    pub paid_amount_centimes: String,
    /// Change owed.
    pub change_amount_centimes: String,
    /// Uncovered remainder (credit-gated at commit, reported here).
    pub remaining_due_centimes: String,
    /// Leaf-level shortages against the requested warehouse (advisory).
    #[serde(default)]
    pub stock_warnings: Vec<StockShortageWarningDto>,
}

/// Draft parking request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDraftSaleRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Customer id, if known (UUID string).
    pub customer_id: Option<String>,
    /// Sale-time receipt language (`AR` or `FR`, mandatory, no default).
    pub document_language: String,
    /// Cart lines.
    pub lines: Vec<CartLineItemDto>,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Draft parking response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSaleResponse {
    /// Parked draft id.
    pub draft_id: String,
    /// Capture instant (RFC3339).
    pub created_at: String,
}

/// Stored document header view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDto {
    /// Document id.
    pub document_id: String,
    /// Human number.
    pub number: String,
    /// Document kind code.
    pub doc_type: String,
    /// Lifecycle status code.
    pub status: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Subtotal HT.
    pub subtotal_ht_centimes: String,
    /// Total TVA.
    pub tva_total_centimes: String,
    /// Grand total.
    pub total_ttc_centimes: String,
    /// Total tendered.
    pub paid_amount_centimes: String,
    /// Change owed.
    pub change_amount_centimes: String,
    /// Derived settlement state (`PAID`, `PARTIAL`, `UNPAID`).
    pub payment_status: String,
}

/// Stored document line view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentItemDto {
    /// Product id.
    pub product_id: String,
    /// Selling unit id.
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Unit price in centimes.
    pub unit_price_centimes: String,
    /// Line TVA.
    pub line_tva_centimes: String,
    /// Line TTC.
    pub line_ttc_centimes: String,
}

/// Stored payment row view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentDto {
    /// Tender method code.
    pub method_code: String,
    /// Tendered amount in centimes.
    pub amount_centimes: String,
}

/// Full sale view response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSaleResponse {
    /// Document header.
    pub document: DocumentDto,
    /// Item snapshots.
    pub items: Vec<DocumentItemDto>,
    /// Payment rows.
    pub payments: Vec<PaymentDto>,
    /// Latest print-job status (`NONE` when absent).
    pub print_status: String,
}

// Customer wire types ---------------------------------------------------
// Ids are UUID strings per the sovereign identity scheme; money travels
// as integer-centime strings.

/// Customer creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCustomerRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Display name (2..=100 chars).
    pub name: String,
    /// Optional unique phone (Algerian format).
    pub phone: Option<String>,
    /// Optional address.
    pub address: Option<String>,
    /// Policy code (`DISABLED`, `LIMITED`, `UNLIMITED`).
    pub credit_policy: String,
    /// Limit in centimes (required when policy is LIMITED).
    pub credit_limit_centimes: Option<String>,
}

/// Customer update request (only `Some` fields are written).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCustomerRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
    /// Replacement display name.
    pub name: Option<String>,
    /// Replacement phone (`Some(None)` clears it).
    pub phone: Option<Option<String>>,
    /// Replacement address (`Some(None)` clears it).
    pub address: Option<Option<String>>,
    /// Replacement policy code.
    pub credit_policy: Option<String>,
    /// Replacement limit in centimes.
    pub credit_limit_centimes: Option<String>,
    /// Replacement active flag.
    pub is_active: Option<bool>,
}

/// Customer list row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerSummaryDto {
    /// Customer id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Policy code.
    pub credit_policy: String,
}

/// Full customer view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerDetailDto {
    /// Customer id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Address, if registered.
    pub address: Option<String>,
    /// Policy code.
    pub credit_policy: String,
    /// Limit in centimes (`None` unless LIMITED).
    pub credit_limit_centimes: Option<String>,
    /// Current ledger balance in centimes.
    pub current_balance_centimes: String,
    /// Active flag.
    pub is_active: bool,
}

/// Customer search request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCustomersRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Prefix/multilingual query (blank returns no rows).
    pub query: String,
    /// Max rows (clamped to 1..=100 server-side).
    pub limit: Option<u32>,
}

/// Customer search response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCustomersResponse {
    /// Matching customers ordered by name.
    pub customers: Vec<CustomerSummaryDto>,
}

/// Single-customer fetch request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetCustomerRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
}

/// Credit payment recording request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordCreditPaymentRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
    /// Payment amount in centimes (must be positive).
    pub amount_centimes: String,
    /// Optional payment reference.
    pub reference: Option<String>,
    /// Allow the balance to go negative (GERANT/ADMIN only).
    pub allow_overpayment: bool,
}

/// Credit payment outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordCreditPaymentResponse {
    /// Stored ledger entry id.
    pub ledger_entry_id: String,
    /// Balance after the payment, in centimes.
    pub new_balance_centimes: String,
}

/// Statement retrieval request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStatementRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
    /// Optional inclusive lower bound (RFC3339).
    pub from_date: Option<String>,
    /// Optional inclusive upper bound (RFC3339).
    pub to_date: Option<String>,
}

/// One statement line with its server-side running balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementEntryDto {
    /// `DEBIT` or `CREDIT`.
    pub entry_type: String,
    /// Entry amount in centimes.
    pub amount_centimes: String,
    /// Referencing document kind, if any.
    pub reference_type: Option<String>,
    /// Referencing document id, if any.
    pub reference_id: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Running balance after this entry, in centimes.
    pub balance_after_centimes: String,
}

/// Statement response (ledger order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStatementResponse {
    /// Statement lines in ledger order.
    pub entries: Vec<StatementEntryDto>,
}

// Print wire types ------------------------------------------------------
// Serial-only transport settings; no USB fields in this task.

/// Latest print-job status view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintStatusResponse {
    /// Stored job id.
    pub job_id: String,
    /// Status code (`PENDING`, `PROCESSING`, `DONE`, `FAILED`).
    pub status: String,
    /// Physical send attempts so far.
    pub attempt_count: String,
}

/// Reprint enqueue request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReprintRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Document id (UUID string).
    pub document_id: String,
}

/// Reprint enqueue response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReprintResponse {
    /// New DUPLICATA job id.
    pub job_id: String,
    /// Echoed document id.
    pub document_id: String,
}

/// Serial printer configuration view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrinterSettingsDto {
    /// COM port name (`""` when unconfigured).
    pub serial_port: String,
    /// Baud rate.
    pub serial_baud: u32,
    /// Paper width in dots.
    pub paper_width_dots: u32,
}

/// Serial printer configuration update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePrinterSettingsRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// COM port name.
    pub serial_port: String,
    /// Baud rate.
    pub serial_baud: u32,
}

// Store identity + diagnostics wire types ---------------------------------
// Persisted as namespaced `store.*` / `printing.device.kind` keys in the
// existing key-value `settings` table — zero schema changes.

/// Store identity view (8 fields; empty = omitted from the header).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreIdentityDto {
    /// Shop display name (required).
    pub store_name: String,
    /// Street address line.
    pub store_address: String,
    /// Phone number.
    pub store_phone: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub nif: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub nis: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub rc: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub ai: String,
    /// Free-text footer (`\n`-separated, up to 3 lines).
    pub ticket_footer_message: String,
}

/// Identity + device load response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreIdentityResponse {
    /// Current identity values.
    pub identity: StoreIdentityDto,
    /// Diagnostics device kind (`physical` | `capture`).
    pub device_kind: String,
}

/// Identity save request (values trimmed server-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveStoreIdentityRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Identity values to persist.
    pub identity: StoreIdentityDto,
    /// Diagnostics device kind (`physical` | `capture`).
    pub device_kind: String,
}

/// One monospace preview line (shared layout blueprint, pre-raster).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewLineDto {
    /// `text` | `rule` | `gap`.
    pub kind: String,
    /// Centered line.
    pub centered: bool,
    /// Pixel em size.
    pub size_px: f32,
    /// Concatenated run texts (`""` for rules/gaps).
    pub text: String,
}

/// Structural verdict over the captured artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodeSummaryDto {
    /// Raster band count.
    pub bands: u32,
    /// Trailing feed lines.
    pub feed_lines: u32,
    /// Unknown-byte spans.
    pub unknown_spans: u32,
}

/// Diagnostics run request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestPrintRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Diagnostic language (`ar` | `fr`).
    pub language: String,
}

/// Diagnostics run response (Amendment 1 §2.3 shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestPrintResponse {
    /// Layout preview from the shared blocks.
    pub preview: Vec<PreviewLineDto>,
    /// Structural verdict over the capture.
    pub decode_summary: DecodeSummaryDto,
    /// Immutable artifact file name.
    pub capture_file_name: String,
    /// `capture-only` | `sent` | typed device-error code.
    pub device_outcome: String,
}

// Cash session wire types -------------------------------------------------
// Money travels as integer-centime strings; ids are UUID strings;
// timestamps are ISO8601 strings. No float ever crosses this boundary.

/// Stored cash session view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionDto {
    /// Session id.
    pub id: String,
    /// Selling warehouse id.
    pub warehouse_id: String,
    /// Terminal identifier.
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
    pub opening_float_centimes: String,
    /// Operator who closed, if closed.
    pub closed_by: Option<String>,
    /// Closing operator username, if closed.
    pub closed_by_username: Option<String>,
    /// Closing instant, if closed.
    pub closed_at: Option<String>,
    /// Physically counted amount, if supplied.
    pub actual_counted_centimes: Option<String>,
    /// Frozen expected total.
    pub system_expected_centimes: Option<String>,
    /// Counted minus expected.
    pub variance_centimes: Option<String>,
    /// Optional closing notes.
    pub closing_notes: Option<String>,
    /// Optimistic concurrency revision.
    pub revision: String,
    /// Row creation instant (RFC3339).
    pub created_at: String,
    /// Row update instant (RFC3339).
    pub updated_at: String,
}

/// Stored cash movement view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovementDto {
    /// Movement id.
    pub id: String,
    /// Owning session.
    pub cash_session_id: String,
    /// Movement type code.
    pub movement_type: String,
    /// Signed amount in centimes.
    pub amount_centimes: String,
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

/// Totals grouped by movement type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovementTotalDto {
    /// Movement type code.
    pub movement_type: String,
    /// Signed total in centimes.
    pub total_centimes: String,
    /// Row count for this type.
    pub count: String,
}

/// Session report (X-Report interim or Z-Report final).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionReportDto {
    /// Session header.
    pub session: CashSessionDto,
    /// Movements in creation order.
    pub movements: Vec<CashMovementDto>,
    /// Totals grouped by movement type.
    pub totals_by_type: Vec<CashMovementTotalDto>,
    /// True when status == CLOSED.
    pub is_final: bool,
}

/// `cash.open` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCashSessionRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id.
    pub warehouse_id: String,
    /// Opening float in centimes.
    pub opening_float_centimes: String,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.recordExpense` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordExpenseRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Expense amount in centimes (must be positive).
    pub amount_centimes: String,
    /// Mandatory description.
    pub description: String,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.confirmClosing` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmClosingRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Session to close.
    pub session_id: String,
    /// Physically counted amount in centimes.
    pub actual_counted_centimes: String,
    /// Optional closing notes.
    pub closing_notes: Option<String>,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.beginClosing` / `cash.cancelClosing` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashClosingCommandDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.getSessionReport` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSessionReportRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Session id.
    pub session_id: String,
}

// Supplier / purchase wire types (TASK-BRK-011) ---------------------------
// Money travels as integer-centime strings; quantities as integer-micro
// strings; ids as UUID strings. No float ever crosses this boundary.

/// Stored supplier view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierDto {
    /// Supplier id.
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
    pub current_balance_centimes: String,
    /// Terminal that created the record (ADR-028).
    pub origin_device_id: String,
    /// Row creation instant (RFC3339).
    pub created_at: String,
    /// Row update instant (RFC3339).
    pub updated_at: String,
}

/// `suppliers.create` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSupplierRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Display name (mandatory).
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

/// `suppliers.update` request (only `Some` fields are written).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSupplierRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Supplier id.
    pub supplier_id: String,
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

/// `suppliers.get` / `suppliers.getStatement` / `purchases.getDocument` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierIdRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target id (supplier or document, per command).
    pub id: String,
}

/// `suppliers.search` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSuppliersRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Prefix/substring query (blank returns no rows).
    pub query: String,
    /// Max rows (clamped 1..=100).
    pub limit: Option<u32>,
}

/// `suppliers.search` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSuppliersResponseDto {
    /// Matching suppliers ordered by name.
    pub suppliers: Vec<SupplierDto>,
}

/// One supplier ledger entry with its server-side running balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierLedgerEntryDto {
    /// Entry id.
    pub id: String,
    /// `DEBIT` or `CREDIT`.
    pub entry_type: String,
    /// Positive magnitude in centimes.
    pub amount_centimes: String,
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
    pub balance_after_centimes: String,
}

/// `suppliers.getStatement` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierStatementDto {
    /// Ledger rows in ledger order.
    pub entries: Vec<SupplierLedgerEntryDto>,
    /// Cached supplier balance for cross-verification.
    pub current_balance_centimes: String,
}

/// One purchase line as submitted (BC/BR/FF payloads).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseLineRequestDto {
    /// Product id.
    pub product_id: String,
    /// Handling unit id.
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Estimated or actual unit cost in centimes.
    pub unit_cost_centimes: String,
}

/// One stored purchase line view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseOrderLineDto {
    /// Stored item id.
    pub id: String,
    /// Product id.
    pub product_id: String,
    /// Handling unit id.
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Quantity in base-unit micros.
    pub quantity_base_micro: String,
    /// Unit cost in centimes.
    pub unit_cost_centimes: String,
    /// Line total in centimes.
    pub line_total_centimes: String,
}

/// Shared purchase-document response shape for BC/BR/FF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseOrderDto {
    /// Document id.
    pub id: String,
    /// Gapless human number.
    pub number: String,
    /// Document type code (`BC`, `BR`, `FF`).
    pub doc_type: String,
    /// Status code.
    pub status: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Predecessor document id, if any.
    pub source_document_id: Option<String>,
    /// Document total in centimes.
    pub total_centimes: String,
    /// Ordered lines.
    pub lines: Vec<PurchaseOrderLineDto>,
    /// Creation instant (RFC3339).
    pub created_at: String,
    /// Creator operator id.
    pub created_by: String,
}

/// `purchases.receiveGoods` response (BR document).
pub type GoodsReceiptDto = PurchaseOrderDto;

/// `purchases.recordInvoice` response (FF document).
pub type SupplierInvoiceDto = PurchaseOrderDto;

/// `purchases.createOrder` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePurchaseOrderRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Receiving warehouse id.
    pub warehouse_id: String,
    /// Ordered lines.
    pub lines: Vec<PurchaseLineRequestDto>,
}

/// `purchases.updateOrder` request (DRAFT only; replaces the line set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePurchaseOrderRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// BC document id.
    pub document_id: String,
    /// Supplier id (must own the BC).
    pub supplier_id: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Replacement line set.
    pub lines: Vec<PurchaseLineRequestDto>,
}

/// `purchases.receiveGoods` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiveGoodsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Source BC id for a receipt against an order (None = standalone).
    pub source_bc_id: Option<String>,
    /// Supplier id.
    pub supplier_id: String,
    /// Receiving warehouse id.
    pub warehouse_id: String,
    /// Received lines.
    pub lines: Vec<PurchaseLineRequestDto>,
}

/// `purchases.recordInvoice` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSupplierInvoiceRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Source BR id (None = standalone combined receipt+invoice, ADR-029).
    pub source_br_id: Option<String>,
    /// Supplier id.
    pub supplier_id: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Invoice lines (must match the BR exactly when sourced).
    pub lines: Vec<PurchaseLineRequestDto>,
    /// Financial total in centimes (the ledger DEBIT amount).
    pub invoice_total_centimes: String,
}

/// `purchases.recordPayment` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSupplierPaymentRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Payment amount in centimes (must be positive).
    pub amount_centimes: String,
    /// `CASH` (drawer outflow, session required) or `OTHER` (ADR-030).
    pub payment_method: String,
    /// Optional description.
    pub description: Option<String>,
}

/// `purchases.recordPayment` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierPaymentOutcomeDto {
    /// Stored supplier_ledger entry id.
    pub ledger_entry_id: String,
    /// Supplier cached balance after the payment, in centimes.
    pub new_balance_centimes: String,
    /// Drawer movement id (CASH payments only, ADR-030).
    pub cash_movement_id: Option<String>,
}

// Reports wire types (TASK-BRK-012) ---------------------------------------
// Read-only aggregates. Money as centime strings, quantities as micro
// strings, ids as UUID strings, dates as YYYY-MM-DD. No float anywhere.

/// Shared date-range request shape for every ranged report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDateRangeRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
}

/// One daily sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailySalesSummaryDto {
    /// Calendar day (`YYYY-MM-DD`).
    pub day: String,
    /// Grand total in centimes.
    pub total_ttc_centimes: String,
    /// HT subtotal in centimes.
    pub total_ht_centimes: String,
    /// TVA total in centimes.
    pub total_tva_centimes: String,
    /// Committed document count.
    pub document_count: String,
    /// Mean basket in centimes.
    pub average_basket_centimes: String,
}

/// One per-product sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesByProductRowDto {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// French display name.
    pub name_fr: String,
    /// Quantity sold in base micros.
    pub quantity_sold_base_micro: String,
    /// Revenue in centimes.
    pub revenue_ttc_centimes: String,
    /// Contributing line count.
    pub line_count: String,
}

/// One per-category sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesByCategoryRowDto {
    /// Category id (`None` = uncategorized products).
    pub category_id: Option<String>,
    /// Category code (`None` = uncategorized).
    pub category_code: Option<String>,
    /// French display name (`None` = uncategorized).
    pub name_fr: Option<String>,
    /// Revenue in centimes.
    pub revenue_ttc_centimes: String,
    /// Contributing line count.
    pub line_count: String,
}

/// One per-product gross-margin row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrossMarginRowDto {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// Revenue HT in centimes.
    pub revenue_ht_centimes: String,
    /// Cost of goods sold in centimes.
    pub cost_total_centimes: String,
    /// Margin in centimes (revenue_ht - cost_total).
    pub margin_centimes: String,
    /// Margin percentage in basis points (margin * 10000 / revenue_ht,
    /// `"0"` when revenue is zero). Display-only, computed server-side.
    pub margin_percent_bp: String,
    /// True when any contributing line had a zero unit cost.
    pub has_untracked_cost: bool,
}

/// One credit-outstanding row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditOutstandingRowDto {
    /// Customer id.
    pub customer_id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Policy code.
    pub credit_policy: String,
    /// Limit in centimes (`None` for UNLIMITED).
    pub credit_limit_centimes: Option<String>,
    /// Current balance in centimes (> 0).
    pub current_balance_centimes: String,
    /// Limit consumed in basis points (`None` for UNLIMITED).
    pub limit_used_bp: Option<String>,
}

/// One stock-movement history row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovementRowDto {
    /// Movement id.
    pub id: String,
    /// Movement type code.
    pub movement_type: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Product id.
    pub product_id: String,
    /// Quantity in base micros (signed delta).
    pub quantity_base_micro: String,
    /// Reference document type, if any.
    pub reference_type: Option<String>,
    /// Reference document id, if any.
    pub reference_id: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator id.
    pub created_by: String,
}

/// `reports.stockMovementHistory` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovementHistoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional warehouse filter.
    pub warehouse_id: Option<String>,
    /// Optional product filter.
    pub product_id: Option<String>,
    /// Optional movement-type filter.
    pub movement_type: Option<String>,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
    /// Page size (default 50, max 200).
    pub limit: Option<u32>,
    /// Row offset.
    pub offset: Option<u32>,
}

/// One cash-session history row (drill down via cash.getSessionReport).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionHistoryRowDto {
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
    pub opening_float_centimes: String,
    /// Frozen expected total, if computed.
    pub system_expected_centimes: Option<String>,
    /// Counted amount, if supplied.
    pub actual_counted_centimes: Option<String>,
    /// Variance, if computed.
    pub variance_centimes: Option<String>,
}

/// `reports.cashSessionsHistory` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionsHistoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional terminal filter.
    pub terminal_id: Option<String>,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
}

/// One user-activity (audit) row (generic payload, never decoded per type).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserActivityRowDto {
    /// Monotonic sequence number.
    pub sequence_no: String,
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

/// `reports.userActivity` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserActivityRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional operator filter.
    pub user_id: Option<String>,
    /// Optional action filter.
    pub action: Option<String>,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
    /// Page size (default 50, max 200).
    pub limit: Option<u32>,
    /// Row offset.
    pub offset: Option<u32>,
}

/// One low-stock row (ADR-037: quantity_base_micro <= 0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowStockRowDto {
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
    /// On-hand quantity in base micros (<= 0).
    pub quantity_base_micro: String,
}

/// `reports.lowStock` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowStockRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional warehouse filter (`None` covers all warehouses).
    pub warehouse_id: Option<String>,
}

/// DEVIS-to-BL conversion request (ADR-040, TASK-BRK-014A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertDevisToBlRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key (UUID v4 string).
    pub command_id: String,
    /// Source DEVIS document id (UUID string).
    pub source_document_id: String,
}

/// DEVIS-to-BL conversion response (stored result shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertDevisToBlResponse {
    /// New BL document id.
    pub new_document_id: String,
    /// New BL gapless human number.
    pub new_document_number: String,
    /// New BL status code (`FINALIZED`).
    pub new_document_status: String,
    /// Copied grand total in centimes.
    pub total_ttc_centimes: String,
    /// Copied line count.
    pub items_count: u32,
    /// Per-line negative-stock override notices (empty when stock sufficed).
    pub warnings: Vec<String>,
}

/// BL-to-FACTURE financial-closing request (ADR-040/041/046, TASK-BRK-038).
///
/// Tender shape reuses the checkout tender DTO verbatim (same method codes,
/// same positive-amount contract); settlement must equal the source total
/// exactly — this closing carries no change concept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertBlToFactureRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key (UUID v4 string).
    pub command_id: String,
    /// Source BL document id (UUID string).
    pub source_document_id: String,
    /// Customer-facing receipt language for the FACTURE (`AR` or `FR`).
    pub document_language: String,
    /// Tender lines covering the source total exactly.
    pub payments: Vec<PaymentLineDto>,
}

/// BL-to-FACTURE financial-closing response (stored result shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertBlToFactureResponse {
    /// New FACTURE document id.
    pub new_document_id: String,
    /// New FACTURE gapless human number.
    pub new_document_number: String,
    /// New FACTURE status code (`FINALIZED`).
    pub new_document_status: String,
    /// Settled grand total in centimes (source copy).
    pub total_ttc_centimes: String,
    /// Copied line count.
    pub items_count: u32,
    /// Customer balance after settlement, when credit was used.
    pub customer_new_balance_centimes: Option<String>,
}

/// Read-only document search request (TASK-BRK-042, conversion lookup).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSearchRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Document kind filter (`DEVIS`, `BL`, `FACTURE`, `TICKET`).
    pub document_type: Option<String>,
    /// Lifecycle status filter (`FINALIZED`, `CONVERTED`, `DRAFT`, …).
    pub status: Option<String>,
    /// Partial match against human number or customer name.
    pub query: Option<String>,
    /// Max rows (default 20, clamped to 1..=100).
    pub limit: Option<u32>,
}

/// Read-only document summary row (all money as canonical strings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSummaryDto {
    /// Document id (UUID string).
    pub id: String,
    /// Gapless human number.
    pub document_number: String,
    /// Document kind code.
    pub document_type: String,
    /// Lifecycle status code.
    pub status: String,
    /// Owning customer id, if any.
    pub party_id: Option<String>,
    /// Owning customer name, if any.
    pub party_name: Option<String>,
    /// Grand total in centimes.
    pub total_ttc_centimes: String,
    /// Stored line count.
    pub items_count: u32,
    /// Creation instant (RFC3339).
    pub created_at: String,
}

/// Read-only document search response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSearchResponse {
    /// Matching summaries, newest first.
    pub documents: Vec<DocumentSummaryDto>,
}

/// One returnable source line (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnableLineDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned product id (UUID string).
    pub product_id: String,
    /// Product display name (FR).
    pub product_name: String,
    /// Stock-keeping reference.
    pub sku: String,
    /// Selling unit id (UUID string).
    pub unit_id: String,
    /// Selling unit display name (FR).
    pub unit_name: String,
    /// Originally sold magnitude in selling-unit micros.
    pub original_qty_micro: String,
    /// Cumulatively returned magnitude in selling-unit micros.
    pub already_returned_qty_micro: String,
    /// Still-returnable magnitude in selling-unit micros.
    pub remaining_qty_micro: String,
    /// Agreed unit price in centimes.
    pub unit_price_cents: String,
    /// Net unit price after pro-rated discount, in centimes.
    pub net_unit_price_cents: String,
    /// Line TVA rate in micros.
    pub tva_rate: String,
}

/// Returnable source document summary (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnableDocumentSummaryDto {
    /// Source document id (UUID string).
    pub source_document_id: String,
    /// Gapless human number.
    pub source_document_number: String,
    /// Document kind code.
    pub document_type: String,
    /// Owning party kind (`CUSTOMER`, or `None` for walk-in sales).
    pub party_type: Option<String>,
    /// Owning customer id, if any.
    pub party_id: Option<String>,
    /// Owning customer name, if any.
    pub party_name: Option<String>,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Source subtotal HT in centimes.
    pub total_ht_cents: String,
    /// Source grand total in centimes.
    pub total_ttc_cents: String,
    /// Convertible now (FINALIZED returnable kind with returnable stock).
    pub is_eligible_for_return: bool,
    /// Per-line returnable state.
    pub items: Vec<ReturnableLineDto>,
}

/// One returned line payload (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnItemPayloadDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned magnitude in selling-unit micros.
    pub quantity_micro: String,
    /// Operator reason, if any.
    pub reason: Option<String>,
}

/// Refund settlement selector (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RefundDestinationDto {
    /// Drawer payout from an OPEN session.
    Cash { cash_session_id: String },
    /// Debt reduction for a named customer.
    CustomerCredit { customer_id: String },
    /// Logistics-only return, no financial leg.
    NoRefund,
}

/// Return execution request (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReturnRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Acting operator id (UUID string; permission gated by the session role).
    pub operator_id: String,
    /// Returned-against source document id (UUID string).
    pub source_document_id: String,
    /// Terminal identifier (must match the edge-resolved terminal).
    pub terminal_id: String,
    /// Returned lines.
    pub items: Vec<ReturnItemPayloadDto>,
    /// Financial settlement.
    pub refund: RefundDestinationDto,
    /// Client idempotency key (UUID string).
    pub idempotency_key: String,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Return execution response (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReturnResponseDto {
    /// New AVOIR document id (UUID string).
    pub return_document_id: String,
    /// New AVOIR gapless human number.
    pub return_document_number: String,
    /// Source document id (UUID string).
    pub source_document_id: String,
    /// Source gapless human number.
    pub source_document_number: String,
    /// Refunded HT in centimes.
    pub total_ht_cents: String,
    /// Refunded TVA in centimes.
    pub total_tva_cents: String,
    /// Refunded TTC in centimes.
    pub total_ttc_cents: String,
    /// Settlement kind (`CASH`, `CUSTOMER_CREDIT`, `NO_REFUND`).
    pub refund_type: String,
    /// Returned line count.
    pub lines_count: usize,
}

/// One returnable source line, eligibility view (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnableItemDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned product id (UUID string).
    pub product_id: String,
    /// Product display name (FR, id fallback).
    pub product_name: String,
    /// Product-level barcode, if any.
    pub barcode: Option<String>,
    /// Selling unit id (UUID string).
    pub unit_id: String,
    /// Selling unit display name (FR, id fallback).
    pub unit_name: String,
    /// Agreed unit price in centimes.
    pub unit_price_centimes: String,
    /// Originally sold magnitude in selling-unit micros.
    pub original_quantity_micro: String,
    /// Cumulatively returned magnitude in selling-unit micros.
    pub already_returned_micro: String,
    /// Still-returnable magnitude in selling-unit micros.
    pub remaining_allowable_micro: String,
    /// Line TVA rate in micros.
    pub tva_rate: u32,
    /// Original line discount in centimes.
    pub discount_centimes: String,
}

/// Return context for one source document (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnContextResponseDto {
    /// Source document id (UUID string).
    pub source_document_id: String,
    /// Gapless human number.
    pub document_number: String,
    /// Document kind code.
    pub document_type: String,
    /// Owning customer id, if any.
    pub party_id: Option<String>,
    /// Owning customer name, if any.
    pub party_name: Option<String>,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Creation instant (RFC3339).
    pub created_at: String,
    /// Convertible now (FINALIZED returnable kind with remaining stock).
    pub is_eligible: bool,
    /// Machine-readable reason when ineligible (`NOT_FINALIZED`,
    /// `NON_RETURNABLE_TYPE`, `FULLY_RETURNED`).
    pub ineligibility_reason: Option<String>,
    /// Per-line returnable state.
    pub items: Vec<ReturnableItemDto>,
}

/// One returned line payload, simplified facade (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnItemInputDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned magnitude in selling-unit micros.
    pub quantity_micro: String,
}

/// Simplified single-shot return request (TASK-BRK-046 re-issue).
///
/// Unlike the explicit `ProcessReturnRequestDto`, settlement travels as a
/// plain method code and session/terminal/idempotency resolve at the edge:
/// operator and terminal come from the session and TerminalContext, and a
/// fresh idempotency key is minted per call (callers needing retry-safety
/// use `returns_process` with an explicit key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentsProcessReturnRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Returned-against source document id (UUID string).
    pub source_document_id: String,
    /// Settlement selector (`CASH`, `CUSTOMER_CREDIT`, `NO_REFUND`).
    pub refund_method: String,
    /// Returned lines.
    pub items: Vec<ReturnItemInputDto>,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Simplified single-shot return response (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentsProcessReturnResponseDto {
    /// New AVOIR document id (UUID string).
    pub return_document_id: String,
    /// New AVOIR gapless human number.
    pub return_document_number: String,
    /// Refunded TTC in centimes.
    pub total_refund_ttc_centimes: String,
    /// Settlement kind as requested (`CASH`, `CUSTOMER_CREDIT`, `NO_REFUND`).
    pub refund_method: String,
}

/// Backup creation request (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBackupRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Trigger selector (`MANUAL`, `SHIFT_CLOSE`, `SCHEDULED`; absent or
    /// blank defaults to `MANUAL`).
    pub trigger_type: Option<String>,
}

/// Backup metadata view (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupMetadataDto {
    /// Backup id (UUID string).
    pub id: String,
    /// Snapshot file name.
    pub file_name: String,
    /// Snapshot size in bytes.
    pub file_size_bytes: u64,
    /// SHA-256 hex over the exact snapshot bytes.
    pub sha256_checksum: String,
    /// Trigger code (`MANUAL`, `SHIFT_CLOSE`, `SCHEDULED`).
    pub trigger_type: String,
    /// Lifecycle state (`SUCCESS`, `FAILED`, `CORRUPTED`, `PRUNED`).
    pub status: String,
    /// Embedded migration count at capture.
    pub schema_version: u32,
    /// Failure detail, if any.
    pub error_message: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
}

/// Backup verification request (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyBackupRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Backup id (file name as logged).
    pub backup_id: String,
}

/// Backup verification outcome (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyBackupResponseDto {
    /// Backup id as requested.
    pub backup_id: String,
    /// True only when the file exists, matches its hash, and passes pragmas.
    pub is_valid: bool,
    /// Live SHA-256 hex over the file bytes.
    pub sha256_checksum: String,
    /// Lifecycle state after verification.
    pub status: String,
    /// Human detail, if any.
    pub message: Option<String>,
}

/// Restore precheck request (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Backup id (file name as logged).
    pub backup_id: String,
}

/// Destructive restore request (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreBackupRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Backup id (file name as logged).
    pub backup_id: String,
}

/// Non-blocking restore advisory view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RestoreWarningDto {
    /// The backup holds a cash session that was never closed.
    OpenCashSessionInBackup {
        /// Session opening instant (RFC3339 UTC).
        opened_at: String,
    },
    /// The backup predates the running binary; migrations run on activate.
    OlderSchemaWillMigrate {
        /// Migration count recorded inside the backup.
        from_version: u32,
        /// Migration count of the running binary.
        to_version: u32,
    },
    /// Live documents newer than the backup survive only in quarantine.
    NewerActivityWillBeQuarantined {
        /// Live documents with `created_at` after the backup instant.
        document_count: u64,
    },
}

/// The eight verification gates view (TASK-BRK-054).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreChecksDto {
    /// Backup file exists on disk and is non-empty.
    pub file_exists: bool,
    /// Streaming SHA-256 matches the recorded binding.
    pub sha256_matches: bool,
    /// `PRAGMA integrity_check` returns exactly `"ok"`.
    pub integrity_ok: bool,
    /// `PRAGMA foreign_key_check` returns zero rows.
    pub foreign_keys_ok: bool,
    /// The full audit hash chain recomputes cleanly.
    pub audit_chain_ok: bool,
    /// Every customer balance equals its signed ledger sum.
    pub customer_ledger_ok: bool,
    /// Every supplier balance equals its signed ledger sum.
    pub supplier_ledger_ok: bool,
    /// Every stock level equals its signed movement sum.
    pub stock_levels_ok: bool,
}

/// Backup content summary view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupContentSummaryDto {
    /// Stored product rows.
    pub products: u64,
    /// Stored customer rows.
    pub customers: u64,
    /// Stored document rows.
    pub documents_total: u64,
    /// Newest document instant, if any (RFC3339 UTC).
    pub last_document_at: Option<String>,
}

/// Restore ineligibility reason view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RestoreIneligibleReasonDto {
    /// Snapshot file missing or zero-length.
    BackupFileMissing,
    /// File bytes drifted from the recorded SHA-256.
    HashMismatch,
    /// Physical page structure is damaged.
    IntegrityCheckFailed,
    /// Dangling references exist.
    ForeignKeyViolation,
    /// The audit hash chain does not recompute.
    AuditChainBroken,
    /// Customer balances diverge from their ledgers.
    CustomerLedgerMismatch {
        /// Violating customer rows.
        mismatch_count: u64,
    },
    /// Supplier balances diverge from their ledgers.
    SupplierLedgerMismatch {
        /// Violating supplier rows.
        mismatch_count: u64,
    },
    /// Stock levels diverge from their movements.
    StockLevelMismatch {
        /// Violating `(warehouse, product)` pairs.
        mismatch_count: u64,
    },
    /// The backup was written by a newer binary; downgrade is forbidden.
    SchemaNewerThanApp {
        /// Migration count recorded inside the backup.
        backup_version: u32,
        /// Migration count of the running binary.
        app_version: u32,
    },
}

/// Full read-only precheck report view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreReportDto {
    /// Logged backup id (snapshot file name).
    pub backup_id: String,
    /// Snapshot file name.
    pub file_name: String,
    /// Snapshot size in bytes (0 when the file is absent).
    pub file_size_bytes: u64,
    /// Capture instant from `backups_log` (RFC3339 UTC).
    pub created_at: String,
    /// Migration count recorded inside the backup.
    pub schema_version_backup: u32,
    /// Migration count of the running binary.
    pub schema_version_app: u32,
    /// The eight verification gates.
    pub checks: PreRestoreChecksDto,
    /// Human-scale content of the backup.
    pub content_summary: BackupContentSummaryDto,
    /// Non-blocking advisories.
    pub warnings: Vec<RestoreWarningDto>,
    /// True only when every check passes and no reason is recorded.
    pub eligible: bool,
    /// Every failed gate, in protocol order.
    pub ineligible_reasons: Vec<RestoreIneligibleReasonDto>,
}

/// Verified restore outcome view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreOutcomeDto {
    /// Logged backup id that was activated.
    pub restored_backup_id: String,
    /// Quarantine base file name of the previous active database.
    pub quarantined_to: String,
    /// Activation instant (RFC3339 UTC).
    pub restored_at: String,
    /// Always true: the operator must restart before resuming work.
    pub restart_required: bool,
}

/// Managed operator view (mirrors application `UserRecord`; never carries hashes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummaryDto {
    /// Operator id (UUID string).
    pub id: String,
    /// Login name.
    pub username: String,
    /// Role code (`VENDEUR`, `CAISSIER`, `GERANT`, `ADMIN`).
    pub role: String,
    /// Active flag.
    pub is_active: bool,
    /// True when a PIN hash slot is configured.
    pub has_pin: bool,
    /// Capture instant (RFC3339 millis).
    pub created_at: String,
    /// Last mutation instant (RFC3339 millis).
    pub updated_at: String,
}

/// `users_create` request (plaintext secrets; hashed inside the use case).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateUserRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Login name (3..=30, alphanumeric + underscores).
    pub username: String,
    /// Plaintext password (min 6 chars).
    pub password: String,
    /// Optional plaintext PIN (4..=6 digits).
    pub pin: Option<String>,
    /// Role code.
    pub role: String,
}

/// `users_update` request (role/active-flag patch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateUserRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target operator id (UUID string).
    pub user_id: String,
    /// New role code, or `None` to keep the current one.
    pub role: Option<String>,
    /// New active flag, or `None` to keep the current one.
    pub is_active: Option<bool>,
}

/// `users_reset_credentials` request (plaintext replacements).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetUserCredentialsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target operator id (UUID string).
    pub user_id: String,
    /// Replacement plaintext password, or `None` to keep the current one.
    pub password: Option<String>,
    /// Replacement plaintext PIN, or `None` to keep the current one.
    pub pin: Option<String>,
}
