//! Sale use cases: atomic checkout, draft parking, sale reads.
//!
//! The checkout orchestrator validates everything it can BEFORE the atomic
//! boundary (RBAC, catalog, pricing, credit policy, kit expansion) and then
//! issues ONE atomic repository call. All SQL writes happen inside that
//! call's `run_transaction`; this module opens no transaction itself.

use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::role::Permission;
use baraka_domain::sales::{
    price_document, CreditPolicy, CustomerId, Document, DocumentId, DocumentItem, DocumentLanguage,
    DocumentStatus, DocumentType, PaymentMethod, PaymentShare, PaymentStatus, PricedDocument,
    PricedLineInput,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::ports::{
    CheckoutCommand, CheckoutPayment, CheckoutReceipt, CheckoutRepository, Clock,
    ConvertBlToFactureCommand, ConvertBlToFactureResult, ConvertDevisToBlCommand,
    ConvertDevisToBlResult, ConvertLeafDeduction, CustomerRepository, DocumentRepository,
    DocumentSearchFilter, DocumentSummary, LeafDeduction, PricedCheckoutLine, ProductRepository,
    SessionContext, StockRepository, StoredPayment, WarehouseRepository,
};
use crate::services::kit_expansion_service::expand_for_movement;
use crate::services::sale_pricing_service::{verify_cart_line, VerifiedLine};

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// One cart line as typed by the caller (strings parsed at the IPC edge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartLineInput {
    /// Product sold (leaf or kit header as displayed).
    pub product_id: ProductId,
    /// Selling unit handled.
    pub unit_id: UnitId,
    /// Quantity in selling-unit micros (must be positive).
    pub quantity_micro: i64,
    /// Explicit operator price (`Some` = override attempt).
    pub unit_price_override: Option<i64>,
    /// Line discount in centimes.
    pub line_discount_centimes: i64,
}

/// One tender line as typed by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentInput {
    /// Tender method.
    pub method: PaymentMethod,
    /// Tendered amount (must be positive).
    pub amount_centimes: i64,
}

/// Catalog-verify and price a cart exactly as the atomic checkout does.
///
/// Single source of truth for sale totals (TASK-BRK-031): the read-only
/// preview ([`PreviewSaleTotalsUseCase`]) and the atomic commit
/// ([`CheckoutSaleUseCase`]) both travel this exact path — per-line
/// catalog verification (override permission gate included), kernel
/// line construction, then the domain `price_document` kernel — so a
/// previewed total can never diverge from the committed one.
pub fn verify_and_price_cart<P>(
    products: &P,
    session: &SessionContext,
    lines: &[CartLineInput],
    global_discount_centimes: i64,
    doc_type: DocumentType,
    payments: &[PaymentInput],
) -> Result<(Vec<VerifiedLine>, PricedDocument), ApplicationError>
where
    P: ProductRepository + ?Sized,
{
    // Catalog verification (+ override gating) per line.
    let mut verified = Vec::with_capacity(lines.len());
    for line in lines {
        if line.unit_price_override.is_some()
            && !session.role.has_permission(Permission::PriceOverride)
        {
            return Err(coded("PRICE_OVERRIDE_UNAUTHORIZED"));
        }
        verified.push(verify_cart_line(
            products,
            line.product_id,
            line.unit_id,
            line.quantity_micro,
            line.unit_price_override,
        )?);
    }
    // Pure pricing (TVA baseline rate is 0 pending the tax engine).
    let kernel_lines: Vec<PricedLineInput> = verified
        .iter()
        .zip(lines.iter())
        .enumerate()
        .map(|(index, (line, raw))| PricedLineInput {
            quantity_micro: line.quantity_micro,
            unit_price_centimes: line.unit_price_centimes,
            line_discount_centimes: raw.line_discount_centimes,
            tva_rate_basis_points: 0,
            sort_order: index as i64,
        })
        .collect();
    let kernel_payments: Vec<PaymentShare> = payments
        .iter()
        .map(|payment| PaymentShare {
            method: payment.method,
            amount_centimes: payment.amount_centimes,
        })
        .collect();
    let priced = price_document(
        &kernel_lines,
        global_discount_centimes,
        doc_type,
        &kernel_payments,
    )
    .map_err(ApplicationError::Validation)?;
    Ok((verified, priced))
}

/// Checkout request (typed; IPC parses wire strings into this shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutInput {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Selling warehouse.
    pub warehouse_id: WarehouseId,
    /// Customer for credit sales, if any.
    pub customer_id: Option<CustomerId>,
    /// Document kind (TICKET, FACTURE, or BL for checkout).
    pub doc_type: DocumentType,
    /// Explicit sale-time receipt language (never defaulted).
    pub language: DocumentLanguage,
    /// Cart lines (non-empty).
    pub lines: Vec<CartLineInput>,
    /// Global discount in centimes.
    pub global_discount_centimes: i64,
    /// Tender lines.
    pub payments: Vec<PaymentInput>,
    /// Explicit in-session confirmation of sale-on-shortage
    /// (TASK-BRK-033). Privileged roles only; silently ignored otherwise.
    pub allow_negative_stock: bool,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Atomic sale orchestrator (pre-validates, then issues one atomic call).
pub struct CheckoutSaleUseCase<'a, P, U, C, W>
where
    P: ?Sized,
    U: ?Sized,
    C: ?Sized,
    W: ?Sized,
{
    products: &'a P,
    customers: &'a U,
    checkout: &'a C,
    warehouses: &'a W,
}

impl<'a, P, U, C, W> CheckoutSaleUseCase<'a, P, U, C, W>
where
    P: ProductRepository + ?Sized,
    U: CustomerRepository + ?Sized,
    C: CheckoutRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(products: &'a P, customers: &'a U, checkout: &'a C, warehouses: &'a W) -> Self {
        Self {
            products,
            customers,
            checkout,
            warehouses,
        }
    }

    /// Validate the sale and commit it atomically.
    /// ADR-025: `terminal_id` is resolved from TerminalContext at the IPC
    /// edge (never client-supplied) and bound into the atomic command.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: CheckoutInput,
        terminal_id: &str,
    ) -> Result<CheckoutReceipt, ApplicationError> {
        if !session.role.has_permission(Permission::CreateSale) {
            return Err(ApplicationError::Unauthorized);
        }
        if input.command_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("command id must not be empty".to_string()),
            ));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let terminal_id = terminal_id.trim().to_string();
        if !matches!(
            input.doc_type,
            DocumentType::Ticket | DocumentType::Facture | DocumentType::Bl
        ) {
            return Err(coded("UNSUPPORTED_DOCUMENT_TYPE"));
        }
        match self.warehouses.get_by_id(input.warehouse_id)? {
            Some(warehouse) if warehouse.is_active => {}
            Some(_) => return Err(coded("WAREHOUSE_INACTIVE")),
            None => return Err(ApplicationError::NotFound("warehouse".to_string())),
        }
        // Catalog verification + pricing — shared with the read-only
        // preview (single source of truth for sale totals, TASK-BRK-031).
        let (verified, priced) = verify_and_price_cart(
            self.products,
            session,
            &input.lines,
            input.global_discount_centimes,
            input.doc_type,
            &input.payments,
        )?;
        // Credit authorization (reads only).
        let mut credit_amount: i128 = 0;
        for payment in &input.payments {
            if payment.method == PaymentMethod::Credit {
                credit_amount = credit_amount
                    .checked_add(i128::from(payment.amount_centimes))
                    .ok_or(DomainError::MoneyOverflow)
                    .map_err(ApplicationError::Validation)?;
            }
        }
        let credit_amount = i64::try_from(credit_amount)
            .map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))?;
        if credit_amount > 0 {
            let customer_id = input
                .customer_id
                .ok_or_else(|| coded("CREDIT_CUSTOMER_REQUIRED"))?;
            let profile = self
                .customers
                .get_credit_profile(customer_id)?
                .ok_or_else(|| ApplicationError::NotFound("customer".to_string()))?;
            if !profile.is_active {
                return Err(ApplicationError::Validation(DomainError::CreditDisabled));
            }
            match profile.policy {
                baraka_domain::sales::CreditPolicy::Disabled => {
                    return Err(ApplicationError::Validation(DomainError::CreditDisabled));
                }
                baraka_domain::sales::CreditPolicy::Limited => {
                    let proposed =
                        i128::from(profile.current_balance_centimes) + i128::from(credit_amount);
                    let proposed = i64::try_from(proposed)
                        .map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))?;
                    if proposed > profile.credit_limit_centimes
                        && !session.role.has_permission(Permission::CreditLimitOverride)
                    {
                        return Err(ApplicationError::CreditLimitExceeded {
                            limit_centimes: profile.credit_limit_centimes,
                            proposed_centimes: proposed,
                        });
                    }
                }
                baraka_domain::sales::CreditPolicy::Unlimited => {}
            }
        }
        // Tender coverage: shortfall is allowed only with credit aboard.
        if priced.remaining_due > 0 && credit_amount == 0 {
            return Err(ApplicationError::Validation(
                DomainError::InsufficientPayment,
            ));
        }
        // Kit expansion uniformly (direct lines pass through as one leaf).
        // Negative-stock authorization is explicit, never silent
        // (TASK-BRK-033): the actor's standing grant AND the operator's
        // in-session confirmation must both hold. Anything short of that
        // reaches the atomic engine unauthorized, where the in-transaction
        // gate hard-blocks it (unprivileged) or demands confirmation
        // (privileged but unconfirmed).
        let negative_override_permitted = session
            .role
            .has_permission(Permission::NegativeStockOverride);
        let authorize_negative = negative_override_permitted && input.allow_negative_stock;
        let mut leaf_deductions = Vec::new();
        for line in &verified {
            for leaf in
                expand_for_movement(self.products, line.product_id, line.quantity_base_micro)?
            {
                leaf_deductions.push(LeafDeduction {
                    product_id: leaf.product_id,
                    unit_id: leaf.base_unit_id,
                    quantity_micro: leaf.quantity_base_micro,
                    authorize_negative_stock: authorize_negative,
                });
            }
        }
        // Priced persistence lines with cost snapshots. Stored discount is
        // the line discount plus the allocated global share.
        let priced_lines: Vec<PricedCheckoutLine> = verified
            .iter()
            .zip(priced.lines.iter())
            .zip(input.lines.iter())
            .enumerate()
            .map(|(index, ((line, priced_line), raw))| PricedCheckoutLine {
                product_id: line.product_id,
                unit_id: line.unit_id,
                quantity_micro: line.quantity_micro,
                quantity_base_micro: line.quantity_base_micro,
                unit_price_centimes: line.unit_price_centimes,
                discount_centimes: raw
                    .line_discount_centimes
                    .saturating_add(priced_line.allocated_global_discount),
                tva_rate_micro: 0,
                line_ht_centimes: priced_line.line_taxable_ht,
                line_tva_centimes: priced_line.line_tva,
                line_ttc_centimes: priced_line.line_ttc,
                cost_price_centimes: line.cost_price_centimes,
                sort_order: index as i64,
            })
            .collect();
        let command = CheckoutCommand {
            request_canonical: canonical_request(&input, &verified, &terminal_id),
            command_id: input.command_id.trim().to_string(),
            warehouse_id: input.warehouse_id,
            terminal_id,
            customer_id: input.customer_id,
            doc_type_code: input.doc_type.code().to_string(),
            document_language: input.language,
            priced_lines,
            leaf_deductions,
            payments: input
                .payments
                .iter()
                .map(|payment| CheckoutPayment {
                    method_code: payment.method.code().to_string(),
                    amount_centimes: payment.amount_centimes,
                })
                .collect(),
            totals: crate::ports::CheckoutTotals {
                subtotal_ht: priced.subtotal_ht,
                total_tva: priced.total_tva,
                global_discount: priced.global_discount,
                timbre: priced.timbre,
                total_ttc: priced.total_ttc,
                paid_total: priced.paid_total,
                change: priced.change,
            },
            credit_amount_centimes: credit_amount,
            credit_override_granted: session.role.has_permission(Permission::CreditLimitOverride),
            allow_negative_stock: input.allow_negative_stock,
            negative_override_permitted,
            actor: session.user_id,
            notes: input.notes.map(|note| note.trim().to_string()),
        };
        self.checkout.checkout_atomic(&command)
    }
}

/// Read-only totals preview input (typed; IPC parses wire strings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewTotalsInput {
    /// Selling warehouse (shortage warnings are read against its levels).
    pub warehouse_id: WarehouseId,
    /// Document kind (same TICKET/FACTURE/BL gate as checkout).
    pub doc_type: DocumentType,
    /// Cart lines (non-empty, else `EmptyCart`).
    pub lines: Vec<CartLineInput>,
    /// Global discount in centimes.
    pub global_discount_centimes: i64,
    /// Tender lines exactly as currently entered by the operator.
    pub payments: Vec<PaymentInput>,
}

/// One leaf-level stock shortage surfaced by the preview (TASK-BRK-033).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockShortageWarning {
    /// Short physical product.
    pub product_id: ProductId,
    /// Commercial designation (FR) for the confirmation banner.
    pub product_name: String,
    /// Stock-keeping reference.
    pub sku: String,
    /// On-hand base micros at preview time (advisory; commit re-checks).
    pub available_qty_micro: i64,
    /// Requested base micros aggregated over the cart.
    pub requested_qty_micro: i64,
}

/// Commit-identical totals preview (all centimes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewTotalsOutput {
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
    /// Uncovered remainder (credit-gated at commit, reported here).
    pub remaining_due: i64,
    /// Leaf-level shortages against the requested warehouse (advisory:
    /// levels may move before commit; the atomic engine re-checks).
    pub stock_warnings: Vec<StockShortageWarning>,
}

/// Read-only totals preview use case (TASK-BRK-031).
///
/// Prices a cart through the exact shared path the atomic checkout travels
/// ([`verify_and_price_cart`] → domain `price_document`) with zero
/// persistence, sequence, stock, ledger, or print side effects. The
/// returned totals are byte-for-byte what a subsequent commit of the same
/// cart state persists — the pre-payment UI never recomputes them.
///
/// TASK-BRK-033 additionally surfaces leaf-level stock shortages against
/// the requested warehouse so the modal can gate the sale before commit.
/// Warnings are advisory (levels may move before commit); the atomic
/// engine performs the authoritative, role-gated re-check.
pub struct PreviewSaleTotalsUseCase<'a, P, S, W>
where
    P: ?Sized,
    S: ?Sized,
    W: ?Sized,
{
    products: &'a P,
    stock: &'a S,
    warehouses: &'a W,
}

impl<'a, P, S, W> PreviewSaleTotalsUseCase<'a, P, S, W>
where
    P: ProductRepository + ?Sized,
    S: StockRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(products: &'a P, stock: &'a S, warehouses: &'a W) -> Self {
        Self {
            products,
            stock,
            warehouses,
        }
    }

    /// Preview the commit-identical totals for the current cart state.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: PreviewTotalsInput,
    ) -> Result<PreviewTotalsOutput, ApplicationError> {
        if !session.role.has_permission(Permission::CreateSale) {
            return Err(ApplicationError::Unauthorized);
        }
        if !matches!(
            input.doc_type,
            DocumentType::Ticket | DocumentType::Facture | DocumentType::Bl
        ) {
            return Err(coded("UNSUPPORTED_DOCUMENT_TYPE"));
        }
        match self.warehouses.get_by_id(input.warehouse_id)? {
            Some(warehouse) if warehouse.is_active => {}
            Some(_) => return Err(coded("WAREHOUSE_INACTIVE")),
            None => return Err(ApplicationError::NotFound("warehouse".to_string())),
        }
        let (verified, priced) = verify_and_price_cart(
            self.products,
            session,
            &input.lines,
            input.global_discount_centimes,
            input.doc_type,
            &input.payments,
        )?;
        // Leaf-level shortage scan: expand kits exactly as the commit
        // does, aggregate requested base micros per leaf product, and
        // compare against current levels (0 when no row exists).
        let mut requested: HashMap<ProductId, i64> = HashMap::new();
        for line in &verified {
            for leaf in
                expand_for_movement(self.products, line.product_id, line.quantity_base_micro)?
            {
                let entry = requested.entry(leaf.product_id).or_insert(0);
                *entry = entry
                    .checked_add(leaf.quantity_base_micro)
                    .ok_or(DomainError::MoneyOverflow)
                    .map_err(ApplicationError::Validation)?;
            }
        }
        let mut stock_warnings = Vec::new();
        for (product_id, requested_qty) in &requested {
            let available = self
                .stock
                .get_stock_level(*product_id, input.warehouse_id)?;
            if available < *requested_qty {
                let (product, _) = self
                    .products
                    .find_by_id(*product_id)?
                    .ok_or_else(|| ApplicationError::NotFound("product".to_string()))?;
                stock_warnings.push(StockShortageWarning {
                    product_id: *product_id,
                    product_name: product.name_fr.clone(),
                    sku: product.sku.as_str().to_string(),
                    available_qty_micro: available,
                    requested_qty_micro: *requested_qty,
                });
            }
        }
        stock_warnings.sort_by(|a, b| a.sku.cmp(&b.sku));
        Ok(PreviewTotalsOutput {
            subtotal_ht: priced.subtotal_ht,
            total_tva: priced.total_tva,
            global_discount: priced.global_discount,
            timbre: priced.timbre,
            total_ttc: priced.total_ttc,
            paid_total: priced.paid_total,
            change: priced.change,
            remaining_due: priced.remaining_due,
            stock_warnings,
        })
    }
}

/// Deterministic canonical request string for idempotency hashing.
///
/// Fixed field order, `;`-joined lines, `""` for absent optionals. The
/// fiscal year is deliberately excluded (infra stamps it; retries across a
/// year boundary must still match).
fn canonical_request(
    input: &CheckoutInput,
    verified: &[crate::services::sale_pricing_service::VerifiedLine],
    terminal_id: &str,
) -> String {
    let mut parts = vec![
        "v1".to_string(),
        input.command_id.trim().to_string(),
        input.warehouse_id.to_string(),
        terminal_id.to_string(),
        input
            .customer_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        input.doc_type.code().to_string(),
        input.language.to_string(),
    ];
    for line in verified {
        parts.push(format!(
            "{}|{}|{}|{}",
            line.product_id, line.unit_id, line.quantity_micro, line.unit_price_centimes
        ));
    }
    parts.push(input.global_discount_centimes.to_string());
    for payment in &input.payments {
        parts.push(format!(
            "{}:{}",
            payment.method.code(),
            payment.amount_centimes
        ));
    }
    // Explicit shortage confirmation is part of the logical command: a
    // replay with a different confirmation stance is a different command.
    parts.push(format!(
        "allow_negative_stock:{}",
        input.allow_negative_stock
    ));
    parts.push(input.notes.clone().unwrap_or_default());
    parts.join(";")
}

/// Draft parking input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftInput {
    /// Selling warehouse.
    pub warehouse_id: WarehouseId,
    /// Customer, if known.
    pub customer_id: Option<CustomerId>,
    /// Explicit sale-time receipt language (never defaulted).
    pub language: DocumentLanguage,
    /// Cart lines.
    pub lines: Vec<CartLineInput>,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Draft parking use case (no sequence consumed, no stock moved).
pub struct CreateDraftSaleUseCase<'a, P, D, W, U, C>
where
    P: ?Sized,
    D: ?Sized,
    W: ?Sized,
    U: ?Sized,
    C: ?Sized,
{
    products: &'a P,
    documents: &'a D,
    warehouses: &'a W,
    customers: &'a U,
    clock: &'a C,
}

impl<'a, P, D, W, U, C> CreateDraftSaleUseCase<'a, P, D, W, U, C>
where
    P: ProductRepository + ?Sized,
    D: DocumentRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
    U: CustomerRepository + ?Sized,
    C: Clock + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(
        products: &'a P,
        documents: &'a D,
        warehouses: &'a W,
        customers: &'a U,
        clock: &'a C,
    ) -> Self {
        Self {
            products,
            documents,
            warehouses,
            customers,
            clock,
        }
    }

    /// Validate and park an editable draft.
    /// `terminal_id` is TerminalContext-resolved (never client-supplied).
    pub fn execute(
        &self,
        session: &SessionContext,
        input: DraftInput,
        terminal_id: &str,
    ) -> Result<DocumentId, ApplicationError> {
        if !session.role.has_permission(Permission::CreateSale) {
            return Err(ApplicationError::Unauthorized);
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        match self.warehouses.get_by_id(input.warehouse_id)? {
            Some(warehouse) if warehouse.is_active => {}
            Some(_) => return Err(coded("WAREHOUSE_INACTIVE")),
            None => return Err(ApplicationError::NotFound("warehouse".to_string())),
        }
        if let Some(customer_id) = input.customer_id {
            if self.customers.get_credit_profile(customer_id)?.is_none() {
                return Err(ApplicationError::NotFound("customer".to_string()));
            }
        }
        let mut priced_inputs = Vec::with_capacity(input.lines.len());
        let mut verified = Vec::with_capacity(input.lines.len());
        for (index, line) in input.lines.iter().enumerate() {
            if line.unit_price_override.is_some()
                && !session.role.has_permission(Permission::PriceOverride)
            {
                return Err(coded("PRICE_OVERRIDE_UNAUTHORIZED"));
            }
            let checked = verify_cart_line(
                self.products,
                line.product_id,
                line.unit_id,
                line.quantity_micro,
                line.unit_price_override,
            )?;
            priced_inputs.push(PricedLineInput {
                quantity_micro: checked.quantity_micro,
                unit_price_centimes: checked.unit_price_centimes,
                line_discount_centimes: line.line_discount_centimes,
                tva_rate_basis_points: 0,
                sort_order: index as i64,
            });
            verified.push(checked);
        }
        let priced = if priced_inputs.is_empty() {
            baraka_domain::sales::pricing::PricedDocument {
                lines: Vec::new(),
                subtotal_ht: 0,
                total_tva: 0,
                global_discount: 0,
                timbre: 0,
                total_ttc: 0,
                paid_total: 0,
                change: 0,
                remaining_due: 0,
            }
        } else {
            price_document(&priced_inputs, 0, DocumentType::Ticket, &[])
                .map_err(ApplicationError::Validation)?
        };
        let now = self.clock.now();
        let id = DocumentId::new();
        let document = Document {
            id,
            language: input.language,
            number: format!(
                "DRAFT-{}",
                Uuid::new_v4().simple().to_string()[..8].to_uppercase()
            ),
            fiscal_year: now.format("%Y").to_string().parse::<i32>().unwrap_or(1970),
            doc_type: DocumentType::Ticket,
            status: DocumentStatus::Draft,
            warehouse_id: input.warehouse_id,
            party_id: input.customer_id.map(|id| id.to_string()),
            subtotal_ht_centimes: priced.subtotal_ht,
            vat_total_centimes: priced.total_tva,
            global_discount_centimes: 0,
            timbre_centimes: 0,
            ttc_centimes: priced.total_ttc,
            paid_amount_centimes: 0,
            change_amount_centimes: 0,
            payment_method: "UNPAID".to_string(),
            notes: input.notes.map(|note| note.trim().to_string()),
            created_at: now,
            created_by: session.user_id,
        };
        let items: Vec<DocumentItem> = verified
            .iter()
            .zip(priced.lines.iter())
            .zip(input.lines.iter())
            .enumerate()
            .map(|(index, ((line, priced_line), raw))| DocumentItem {
                id: Uuid::new_v4(),
                document_id: id,
                product_id: line.product_id,
                unit_id: line.unit_id,
                quantity_micro: line.quantity_micro,
                quantity_base_micro: line.quantity_base_micro,
                unit_price_centimes: line.unit_price_centimes,
                discount_centimes: raw.line_discount_centimes,
                tva_rate_micro: 0,
                line_ht_centimes: priced_line.line_taxable_ht,
                line_tva_centimes: priced_line.line_tva,
                line_ttc_centimes: priced_line.line_ttc,
                cost_price_centimes: line.cost_price_centimes,
                sort_order: index as i64,
            })
            .collect();
        self.documents
            .create_draft(&document, &items, terminal_id.trim())?;
        Ok(id)
    }
}

/// Full sale view with derived settlement state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleDetail {
    /// Document header.
    pub document: Document,
    /// Item snapshots in display order.
    pub items: Vec<DocumentItem>,
    /// Payment rows.
    pub payments: Vec<StoredPayment>,
    /// Latest print-job status (`NONE` when absent).
    pub print_status: String,
    /// Derived settlement state.
    pub payment_status: PaymentStatus,
}

/// Single-shot DEVIS-to-BL conversion input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertDevisToBlInput {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Source DEVIS document id.
    pub source_document_id: DocumentId,
}

/// Single-shot DEVIS-to-BL converter (ADR-040).
///
/// Pre-validates RBAC, source identity/type/status, non-emptiness, and kit
/// expansion BEFORE the atomic boundary, then issues ONE atomic repository
/// call. The repository re-validates the status strictly inside the
/// transaction (BEGIN IMMEDIATE closes the check-then-act race).
pub struct ConvertDevisToBlUseCase<'a, P, D>
where
    P: ?Sized,
    D: ?Sized,
{
    products: &'a P,
    documents: &'a D,
}

impl<'a, P, D> ConvertDevisToBlUseCase<'a, P, D>
where
    P: ProductRepository + ?Sized,
    D: DocumentRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(products: &'a P, documents: &'a D) -> Self {
        Self {
            products,
            documents,
        }
    }

    /// Validate the conversion and commit it atomically.
    /// ADR-028: `terminal_id` is resolved from TerminalContext at the IPC
    /// edge (never client-supplied) and bound into the atomic command.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: ConvertDevisToBlInput,
        terminal_id: &str,
    ) -> Result<ConvertDevisToBlResult, ApplicationError> {
        // Same permission as standalone BL creation (CheckoutSaleUseCase).
        if !session.role.has_permission(Permission::CreateSale) {
            return Err(ApplicationError::Unauthorized);
        }
        if input.command_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("command id must not be empty".to_string()),
            ));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let terminal_id = terminal_id.trim().to_string();
        // Source identity (authoritative re-check happens in-transaction).
        let source = self.documents.get_by_id(input.source_document_id)?.ok_or(
            ApplicationError::Validation(DomainError::SourceDocumentNotFound),
        )?;
        // This conversion consumes DEVIS only; BL sources belong to the
        // future BL-to-FACTURE conversion (TASK-BRK-014B).
        if source.doc_type != DocumentType::Devis {
            return Err(ApplicationError::Validation(
                DomainError::InvalidConversionSourceType,
            ));
        }
        // Status pre-check defers CONVERTED to the repository: a replayed
        // command must reach the in-transaction idempotency lookup even
        // though the source now reads CONVERTED. All other mismatches fail
        // fast here AND authoritatively in-transaction.
        match DocumentStatus::validate_conversion_transition(source.status, source.doc_type) {
            Ok(()) => {}
            Err(DomainError::DocumentAlreadyConverted) => {}
            Err(other) => return Err(ApplicationError::Validation(other)),
        }
        let items = self.documents.get_items(input.source_document_id)?;
        if items.is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::EmptySourceDocument,
            ));
        }
        // Kit expansion uniformly (direct lines pass through as one leaf).
        let mut leaf_deductions = Vec::new();
        for item in &items {
            for leaf in
                expand_for_movement(self.products, item.product_id, item.quantity_base_micro)?
            {
                leaf_deductions.push(ConvertLeafDeduction {
                    product_id: leaf.product_id,
                    unit_id: leaf.base_unit_id,
                    quantity_base_micro: leaf.quantity_base_micro,
                });
            }
        }
        let command = ConvertDevisToBlCommand {
            request_canonical: canonical_conversion_request(&input, &source, &items, &terminal_id),
            command_id: input.command_id.trim().to_string(),
            source_document_id: input.source_document_id,
            terminal_id,
            leaf_deductions,
            authorize_negative_stock: session
                .role
                .has_permission(Permission::NegativeStockOverride),
            actor: session.user_id,
        };
        self.documents.convert_devis_to_bl(&command)
    }
}

/// Deterministic canonical request string for conversion idempotency.
///
/// Fixed field order, `;`-joined lines. The fiscal year is deliberately
/// excluded (infra stamps it; retries across a year boundary must match).
fn canonical_conversion_request(
    input: &ConvertDevisToBlInput,
    source: &Document,
    items: &[DocumentItem],
    terminal_id: &str,
) -> String {
    let mut parts = vec![
        "v1".to_string(),
        "convert-devis-to-bl".to_string(),
        input.command_id.trim().to_string(),
        input.source_document_id.to_string(),
        source.warehouse_id.to_string(),
        terminal_id.to_string(),
    ];
    for item in items {
        parts.push(format!(
            "{}|{}|{}|{}|{}|{}",
            item.product_id,
            item.unit_id,
            item.quantity_micro,
            item.unit_price_centimes,
            item.discount_centimes,
            item.sort_order,
        ));
    }
    parts.join(";")
}

/// One tender line for financial closing, reusing the checkout tender shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingPaymentInput {
    /// Tender method.
    pub method: PaymentMethod,
    /// Tendered amount (must be positive).
    pub amount_centimes: i64,
}

/// Financial-closing request (typed; IPC parses wire strings into this shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingInput {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Source BL document id.
    pub source_document_id: DocumentId,
    /// Customer-facing receipt language for the FACTURE (never defaulted).
    pub language: DocumentLanguage,
    /// Tender lines covering the source total exactly (no change concept).
    pub payments: Vec<ClosingPaymentInput>,
}

/// Single-shot BL-to-FACTURE financial closer (ADR-040/041/046).
///
/// Pre-validates RBAC, source identity/type/status, non-emptiness, and the
/// exact tender settlement (reusing checkout's payment/credit validation
/// semantics line-for-line) BEFORE the atomic boundary, then issues ONE
/// atomic repository call. The repository performs zero stock writes by
/// invariant (ADR-046): physical stock left the warehouse when the BL was
/// finalized.
pub struct ConvertBlToFactureUseCase<'a, U, D>
where
    U: ?Sized,
    D: ?Sized,
{
    customers: &'a U,
    documents: &'a D,
}

impl<'a, U, D> ConvertBlToFactureUseCase<'a, U, D>
where
    U: CustomerRepository + ?Sized,
    D: DocumentRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(customers: &'a U, documents: &'a D) -> Self {
        Self {
            customers,
            documents,
        }
    }

    /// Validate the closing and commit it atomically.
    /// ADR-045: `terminal_id` is a plain parameter resolved from
    /// TerminalContext at the IPC edge (never client-supplied).
    pub fn execute(
        &self,
        session: &SessionContext,
        input: ClosingInput,
        terminal_id: &str,
    ) -> Result<ConvertBlToFactureResult, ApplicationError> {
        // Same permission as standalone FACTURE creation (CheckoutSaleUseCase).
        if !session.role.has_permission(Permission::CreateSale) {
            return Err(ApplicationError::Unauthorized);
        }
        if input.command_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("command id must not be empty".to_string()),
            ));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let terminal_id = terminal_id.trim().to_string();
        if input.payments.is_empty() {
            return Err(ApplicationError::Validation(DomainError::EmptyCart));
        }
        for payment in &input.payments {
            if payment.amount_centimes <= 0 {
                return Err(coded("INVALID_PAYMENT_AMOUNT"));
            }
        }
        // Source identity (authoritative re-check happens in-transaction).
        let source = self.documents.get_by_id(input.source_document_id)?.ok_or(
            ApplicationError::Validation(DomainError::SourceDocumentNotFound),
        )?;
        // This closing consumes BL only; DEVIS sources belong to the
        // DEVIS-to-BL conversion (TASK-BRK-014A).
        if source.doc_type != DocumentType::Bl {
            return Err(ApplicationError::Validation(
                DomainError::InvalidConversionSourceType,
            ));
        }
        // Status pre-check defers CONVERTED to the repository: a replayed
        // command must reach the in-transaction idempotency lookup even
        // though the source now reads CONVERTED.
        match DocumentStatus::validate_conversion_transition(source.status, source.doc_type) {
            Ok(()) => {}
            Err(DomainError::DocumentAlreadyConverted) => {}
            Err(other) => return Err(ApplicationError::Validation(other)),
        }
        let items = self.documents.get_items(input.source_document_id)?;
        if items.is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::EmptySourceDocument,
            ));
        }
        // Exact tender settlement: the invoice closes fully or not at all.
        // Same mismatch error as checkout (no change concept exists here).
        let mut tendered: i128 = 0;
        let mut credit_amount: i128 = 0;
        for payment in &input.payments {
            tendered = tendered
                .checked_add(i128::from(payment.amount_centimes))
                .ok_or(DomainError::MoneyOverflow)
                .map_err(ApplicationError::Validation)?;
            if payment.method == PaymentMethod::Credit {
                credit_amount = credit_amount
                    .checked_add(i128::from(payment.amount_centimes))
                    .ok_or(DomainError::MoneyOverflow)
                    .map_err(ApplicationError::Validation)?;
            }
        }
        if tendered != i128::from(source.ttc_centimes) {
            return Err(ApplicationError::Validation(
                DomainError::InsufficientPayment,
            ));
        }
        let credit_amount = i64::try_from(credit_amount)
            .map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))?;
        // Credit authorization (reads only; mirrors CheckoutSaleUseCase).
        if credit_amount > 0 {
            let customer_id = source
                .party_id
                .as_deref()
                .and_then(|text| {
                    uuid::Uuid::parse_str(text)
                        .map(baraka_domain::sales::CustomerId::from)
                        .ok()
                })
                .ok_or_else(|| coded("CREDIT_CUSTOMER_REQUIRED"))?;
            let profile = self
                .customers
                .get_credit_profile(customer_id)?
                .ok_or_else(|| ApplicationError::NotFound("customer".to_string()))?;
            if !profile.is_active {
                return Err(ApplicationError::Validation(DomainError::CreditDisabled));
            }
            match profile.policy {
                CreditPolicy::Disabled => {
                    return Err(ApplicationError::Validation(DomainError::CreditDisabled));
                }
                CreditPolicy::Limited => {
                    let proposed =
                        i128::from(profile.current_balance_centimes) + i128::from(credit_amount);
                    let proposed = i64::try_from(proposed)
                        .map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))?;
                    if proposed > profile.credit_limit_centimes
                        && !session.role.has_permission(Permission::CreditLimitOverride)
                    {
                        return Err(ApplicationError::CreditLimitExceeded {
                            limit_centimes: profile.credit_limit_centimes,
                            proposed_centimes: proposed,
                        });
                    }
                }
                CreditPolicy::Unlimited => {}
            }
        }
        let command = ConvertBlToFactureCommand {
            request_canonical: canonical_closing_request(&input, &source, &terminal_id),
            command_id: input.command_id.trim().to_string(),
            source_document_id: input.source_document_id,
            terminal_id,
            document_language: input.language,
            payments: input
                .payments
                .iter()
                .map(|payment| CheckoutPayment {
                    method_code: payment.method.code().to_string(),
                    amount_centimes: payment.amount_centimes,
                })
                .collect(),
            credit_amount_centimes: credit_amount,
            credit_override_granted: session.role.has_permission(Permission::CreditLimitOverride),
            actor: session.user_id,
        };
        self.documents.convert_bl_to_facture(&command)
    }
}

/// Deterministic canonical request string for closing idempotency.
fn canonical_closing_request(input: &ClosingInput, source: &Document, terminal_id: &str) -> String {
    let mut parts = vec![
        "v1".to_string(),
        "convert-bl-to-facture".to_string(),
        input.command_id.trim().to_string(),
        input.source_document_id.to_string(),
        source.warehouse_id.to_string(),
        terminal_id.to_string(),
        input.language.to_string(),
    ];
    for payment in &input.payments {
        parts.push(format!(
            "{}:{}",
            payment.method.code(),
            payment.amount_centimes
        ));
    }
    parts.join(";")
}

/// Parked-draft discard use case (DRAFT -> CANCELLED, no stock touched).
pub struct DiscardDraftSaleUseCase<'a, D>
where
    D: ?Sized,
{
    documents: &'a D,
}

impl<'a, D> DiscardDraftSaleUseCase<'a, D>
where
    D: DocumentRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(documents: &'a D) -> Self {
        Self { documents }
    }

    /// Validate and discard a parked draft.
    pub fn execute(
        &self,
        session: &SessionContext,
        draft_id: &str,
    ) -> Result<(), ApplicationError> {
        if !session.role.has_permission(Permission::CreateSale) {
            return Err(ApplicationError::Unauthorized);
        }
        let id =
            DocumentId(Uuid::parse_str(draft_id.trim()).map_err(|_| coded("INVALID_DOCUMENT_ID"))?);
        let document = self
            .documents
            .get_by_id(id)?
            .ok_or_else(|| ApplicationError::NotFound("draft".to_string()))?;
        if document.status != DocumentStatus::Draft {
            return Err(ApplicationError::Conflict(
                "document is not a draft".to_string(),
            ));
        }
        self.documents.cancel_draft(id)
    }
}

/// Sale read use case (any authenticated session; enforced at the edge).
pub struct GetSaleUseCase<'a, D>
where
    D: ?Sized,
{
    documents: &'a D,
}

impl<'a, D> GetSaleUseCase<'a, D>
where
    D: DocumentRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(documents: &'a D) -> Self {
        Self { documents }
    }

    /// Load the full sale view.
    pub fn execute(
        &self,
        _session: &SessionContext,
        id: DocumentId,
    ) -> Result<SaleDetail, ApplicationError> {
        let document = self
            .documents
            .get_by_id(id)?
            .ok_or_else(|| ApplicationError::NotFound("document".to_string()))?;
        let items = self.documents.get_items(id)?;
        let payments = self.documents.get_payments(id)?;
        let print_status = self.documents.get_print_status(id)?;
        let paid: i128 = payments.iter().map(|p| i128::from(p.amount_centimes)).sum();
        let has_credit = payments.iter().any(|p| p.method_code == "CREDIT");
        let paid_capped = i64::try_from(paid).unwrap_or(i64::MAX);
        Ok(SaleDetail {
            document: document.clone(),
            items,
            payments,
            print_status,
            payment_status: PaymentStatus::derive(paid_capped, document.ttc_centimes, has_credit),
        })
    }
}

/// Read-only document search (conversion candidate lookup, TASK-BRK-042).
///
/// Any operator who may sell or report may search (mirrors the conversion
/// commands' `CreateSale` gate, extended to reporting viewers). Filter
/// codes are validated strictly; the row limit is clamped to 1..=100
/// (20 when unset or zero).
pub struct SearchDocumentsUseCase<'a, D>
where
    D: ?Sized,
{
    documents: &'a D,
}

impl<'a, D> SearchDocumentsUseCase<'a, D>
where
    D: DocumentRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(documents: &'a D) -> Self {
        Self { documents }
    }

    /// Validate the filter and run the read-only search.
    pub fn execute(
        &self,
        session: &SessionContext,
        filter: DocumentSearchFilter,
    ) -> Result<Vec<DocumentSummary>, ApplicationError> {
        if !session.role.has_permission(Permission::CreateSale)
            && !session.role.has_permission(Permission::ReportsView)
        {
            return Err(ApplicationError::Unauthorized);
        }
        if let Some(doc_type) = filter.doc_type.as_deref() {
            DocumentType::from_code(doc_type.trim()).map_err(ApplicationError::Validation)?;
        }
        if let Some(status) = filter.status.as_deref() {
            DocumentStatus::from_code(status.trim()).map_err(ApplicationError::Validation)?;
        }
        let limit = match filter.limit.unwrap_or(20) {
            0 => 20,
            over if over > 100 => 100,
            capped => capped,
        };
        let query = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        self.documents.search_documents(&DocumentSearchFilter {
            doc_type: filter
                .doc_type
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            status: filter.status.as_deref().map(str::trim).map(str::to_string),
            query,
            limit: Some(limit),
        })
    }
}
