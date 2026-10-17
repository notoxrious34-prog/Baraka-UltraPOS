//! Catalog use cases: products, units, kits, search.
//!
//! Mutations require `StockAdjustment` or `SystemSettingsManage`
//! (Gerant/Admin). Reads require `PosSell` (all roles).

use baraka_domain::catalog::{
    validate_kit_edges, validate_unit_set, Barcode, Category, CategoryId, KitItem, Product,
    ProductId, ProductUnit, Sku, UnitId,
};
use baraka_domain::errors::DomainError;
use baraka_domain::primitives::{FactorMicro, Money, QtyMicro};
use baraka_domain::role::Permission;

use crate::errors::ApplicationError;
use crate::ports::{CategoryRepository, ProductRepository, SessionContext};

/// Roles allowed to mutate the catalog.
pub(crate) const MANAGE_PERMISSIONS: [Permission; 2] = [
    Permission::StockAdjustment,
    Permission::SystemSettingsManage,
];

pub(crate) fn require_any(
    session: &SessionContext,
    permissions: &[Permission],
) -> Result<(), ApplicationError> {
    if permissions
        .iter()
        .any(|permission| session.role.has_permission(*permission))
    {
        Ok(())
    } else {
        Err(ApplicationError::Unauthorized)
    }
}

fn require(session: &SessionContext, permission: Permission) -> Result<(), ApplicationError> {
    require_any(session, &[permission])
}

/// One selling/base unit supplied at product creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUnitInput {
    /// Arabic unit name.
    pub unit_name_ar: String,
    /// French unit name.
    pub unit_name_fr: String,
    /// Base micros per selling unit (`10_000` for the base unit).
    pub factor_micro: i64,
    /// Optional barcode resolving to this unit.
    pub barcode: Option<String>,
    /// Sell price in centimes.
    pub sale_price_centimes: i64,
    /// Purchase price in centimes.
    pub purchase_price_centimes: i64,
    /// Artisan/wholesale price in centimes (`None` = fall back to sale price).
    pub wholesale_price_centimes: Option<i64>,
    /// True for the single base unit.
    pub is_base_unit: bool,
    /// Default POS unit.
    pub is_default_selling: bool,
}

/// Product creation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProductInput {
    /// Unique business reference.
    pub sku: String,
    /// Optional product-level barcode.
    pub barcode: Option<String>,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category.
    pub category_id: Option<CategoryId>,
    /// True when the product is a kit header.
    pub is_kit: bool,
    /// Full unit set (exactly one base unit).
    pub units: Vec<NewUnitInput>,
}

/// Unit upsert for product updates (`None` id allocates a new unit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitUpsert {
    /// Existing unit id, or `None` for a brand-new unit.
    pub id: Option<UnitId>,
    /// Replacement unit fields.
    pub unit: NewUnitInput,
}

/// Product metadata + unit-set update payload (SKU stays immutable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateProductInput {
    /// Product to update.
    pub id: ProductId,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category.
    pub category_id: Option<CategoryId>,
    /// Active flag.
    pub is_active: bool,
    /// Merged unit set after applying upserts.
    pub units: Vec<UnitUpsert>,
}

/// One BOM line supplied at kit definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewKitItemInput {
    /// Component product.
    pub component_id: ProductId,
    /// Component unit the quantity is expressed in.
    pub unit_id: UnitId,
    /// Quantity per kit, in unit micros (must be positive).
    pub quantity_micro: i64,
}

/// Product with its units and, for kits, its BOM lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDetail {
    /// The product itself.
    pub product: Product,
    /// All associated units.
    pub units: Vec<ProductUnit>,
    /// BOM lines (empty for non-kits).
    pub kit_items: Vec<KitItem>,
}

fn build_unit(
    product_id: ProductId,
    id: UnitId,
    input: &NewUnitInput,
) -> Result<ProductUnit, ApplicationError> {
    if input.unit_name_ar.trim().is_empty() || input.unit_name_fr.trim().is_empty() {
        return Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString("unit names must not be empty".to_string()),
        ));
    }
    if input.sale_price_centimes < 0 || input.purchase_price_centimes < 0 {
        return Err(ApplicationError::Validation(
            DomainError::NegativeMoneyForbidden,
        ));
    }
    if input
        .wholesale_price_centimes
        .is_some_and(|price| price < 0)
    {
        return Err(ApplicationError::Validation(
            DomainError::NegativeMoneyForbidden,
        ));
    }
    let unit_barcode = input
        .barcode
        .as_deref()
        .map(Barcode::new)
        .transpose()
        .map_err(ApplicationError::Validation)?;
    Ok(ProductUnit {
        id,
        product_id,
        unit_name_ar: input.unit_name_ar.trim().to_string(),
        unit_name_fr: input.unit_name_fr.trim().to_string(),
        factor_micro: FactorMicro::from_micros(input.factor_micro),
        barcode: unit_barcode,
        sale_price: Money::from_centimes(input.sale_price_centimes),
        purchase_price: Money::from_centimes(input.purchase_price_centimes),
        wholesale_price: input.wholesale_price_centimes.map(Money::from_centimes),
        is_base_unit: input.is_base_unit,
        is_default_selling: input.is_default_selling,
    })
}

/// Create a product with its full unit set atomically.
pub struct CreateProductUseCase<'a, P, C>
where
    P: ?Sized,
    C: ?Sized,
{
    products: &'a P,
    categories: &'a C,
}

impl<'a, P, C> CreateProductUseCase<'a, P, C>
where
    P: ProductRepository + ?Sized,
    C: CategoryRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(products: &'a P, categories: &'a C) -> Self {
        Self {
            products,
            categories,
        }
    }

    /// Validate, build, and persist a product with its units.
    /// `terminal_id` is TerminalContext-resolved (never client-supplied).
    pub fn execute(
        &self,
        session: &SessionContext,
        input: CreateProductInput,
        terminal_id: &str,
    ) -> Result<ProductId, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        if terminal_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_TERMINAL_ID".to_string()),
            ));
        }
        let sku = Sku::new(&input.sku).map_err(ApplicationError::Validation)?;
        let barcode = input
            .barcode
            .as_deref()
            .map(Barcode::new)
            .transpose()
            .map_err(ApplicationError::Validation)?;
        Product::validate_names(&input.name_ar, &input.name_fr)
            .map_err(ApplicationError::Validation)?;
        if let Some(category_id) = input.category_id {
            if self.categories.find_category_by_id(category_id)?.is_none() {
                return Err(ApplicationError::NotFound("category".to_string()));
            }
        }
        let id = ProductId::new();
        let mut units = Vec::with_capacity(input.units.len());
        for unit in &input.units {
            units.push(build_unit(id, UnitId::new(), unit)?);
        }
        validate_unit_set(id, &units).map_err(ApplicationError::Validation)?;
        let product = Product {
            id,
            sku,
            barcode,
            name_ar: input.name_ar.trim().to_string(),
            name_fr: input.name_fr.trim().to_string(),
            category_id: input.category_id,
            is_kit: input.is_kit,
            is_active: true,
        };
        self.products
            .create_product(&product, &units, terminal_id.trim())?;
        Ok(id)
    }
}

/// Update product metadata and merge its unit set (SKU immutable).
pub struct UpdateProductUseCase<'a, P, C>
where
    P: ?Sized,
    C: ?Sized,
{
    products: &'a P,
    categories: &'a C,
}

impl<'a, P, C> UpdateProductUseCase<'a, P, C>
where
    P: ProductRepository + ?Sized,
    C: CategoryRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(products: &'a P, categories: &'a C) -> Self {
        Self {
            products,
            categories,
        }
    }

    /// Apply metadata changes and unit upserts atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: UpdateProductInput,
    ) -> Result<(), ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        let (mut product, existing_units) = match self.products.find_by_id(input.id)? {
            Some(found) => found,
            None => return Err(ApplicationError::NotFound("product".to_string())),
        };
        Product::validate_names(&input.name_ar, &input.name_fr)
            .map_err(ApplicationError::Validation)?;
        if let Some(category_id) = input.category_id {
            if self.categories.find_category_by_id(category_id)?.is_none() {
                return Err(ApplicationError::NotFound("category".to_string()));
            }
        }
        let mut merged = Vec::with_capacity(input.units.len());
        for upsert in &input.units {
            let id = match upsert.id {
                Some(id) if existing_units.iter().any(|unit| unit.id == id) => id,
                _ => UnitId::new(),
            };
            merged.push(build_unit(product.id, id, &upsert.unit)?);
        }
        validate_unit_set(product.id, &merged).map_err(ApplicationError::Validation)?;
        product.name_ar = input.name_ar.trim().to_string();
        product.name_fr = input.name_fr.trim().to_string();
        product.category_id = input.category_id;
        product.is_active = input.is_active;
        self.products.update_product(&product, &merged)?;
        Ok(())
    }
}

/// Replace a kit's component graph after acyclic validation.
pub struct DefineKitUseCase<'a, P>
where
    P: ?Sized,
{
    products: &'a P,
}

impl<'a, P> DefineKitUseCase<'a, P>
where
    P: ProductRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(products: &'a P) -> Self {
        Self { products }
    }

    /// Validate structure, activity, and acyclicity, then persist.
    pub fn execute(
        &self,
        session: &SessionContext,
        kit_id: ProductId,
        items: Vec<NewKitItemInput>,
    ) -> Result<(), ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        let (kit, _) = match self.products.find_by_id(kit_id)? {
            Some(found) => found,
            None => return Err(ApplicationError::NotFound("kit product".to_string())),
        };
        if !kit.is_kit {
            return Err(ApplicationError::Validation(
                DomainError::InvalidKitComponent,
            ));
        }
        if items.is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidKitComponent,
            ));
        }
        let mut kit_items = Vec::with_capacity(items.len());
        let mut components = Vec::with_capacity(items.len());
        for item in &items {
            let (component, component_units) = match self.products.find_by_id(item.component_id)? {
                Some(found) => found,
                None => {
                    return Err(ApplicationError::Validation(
                        DomainError::InvalidKitComponent,
                    ));
                }
            };
            if !component.is_active {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidKitComponent,
                ));
            }
            if !component_units.iter().any(|unit| unit.id == item.unit_id) {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidKitComponent,
                ));
            }
            if item.quantity_micro <= 0 {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidKitComponent,
                ));
            }
            components.push(item.component_id);
            kit_items.push(KitItem {
                kit_product_id: kit_id,
                component_product_id: item.component_id,
                unit_id: item.unit_id,
                quantity_micro: QtyMicro::from_micros(item.quantity_micro),
            });
        }
        // Stored edges of *other* kits only: this kit's previous graph is
        // being replaced, so its old edges must not poison validation.
        let stored: Vec<(ProductId, ProductId)> = self
            .products
            .get_all_kit_edges()?
            .into_iter()
            .filter(|(from, _)| *from != kit_id)
            .collect();
        validate_kit_edges(kit_id, &components, &stored).map_err(ApplicationError::Validation)?;
        self.products.define_kit(kit_id, &kit_items)?;
        Ok(())
    }
}

/// Fetch a product with units and BOM breakdown (kits only).
pub struct GetProductWithUnitsUseCase<'a, P>
where
    P: ?Sized,
{
    products: &'a P,
}

impl<'a, P> GetProductWithUnitsUseCase<'a, P>
where
    P: ProductRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(products: &'a P) -> Self {
        Self { products }
    }

    /// Load the full product detail view.
    pub fn execute(
        &self,
        session: &SessionContext,
        id: ProductId,
    ) -> Result<ProductDetail, ApplicationError> {
        require(session, Permission::PosSell)?;
        let (product, units) = match self.products.find_by_id(id)? {
            Some(found) => found,
            None => return Err(ApplicationError::NotFound("product".to_string())),
        };
        let kit_items = if product.is_kit {
            self.products.get_kit_items(id)?
        } else {
            Vec::new()
        };
        Ok(ProductDetail {
            product,
            units,
            kit_items,
        })
    }
}

/// Barcode-exact and multilingual prefix search.
pub struct SearchCatalogUseCase<'a, P>
where
    P: ?Sized,
{
    products: &'a P,
}

impl<'a, P> SearchCatalogUseCase<'a, P>
where
    P: ProductRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(products: &'a P) -> Self {
        Self { products }
    }

    /// Search the catalog (blank queries return default discovery rows).
    pub fn execute(
        &self,
        session: &SessionContext,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(Product, Vec<ProductUnit>)>, ApplicationError> {
        require(session, Permission::PosSell)?;
        let trimmed = query.trim();
        self.products.search(trimmed, limit.clamp(1, 100))
    }
}

/// Category creation payload (active by default; code unique).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCategoryInput {
    /// Unique business code, e.g. `CAT-PLOMB`.
    pub code: String,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category, if nested.
    pub parent_id: Option<CategoryId>,
}

/// Category update payload (code stays immutable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCategoryInput {
    /// Category to update.
    pub id: CategoryId,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category, if nested.
    pub parent_id: Option<CategoryId>,
    /// Active flag.
    pub is_active: bool,
}

/// Deterministic Algerian hardware starter taxonomy: (code, name_fr, name_ar).
pub const STARTER_CATEGORIES: [(&str, &str, &str); 7] = [
    ("CAT-PLOMB", "Plomberie & Sanitaire", "الترصيص الصحي والغاز"),
    ("CAT-ELEC", "Électricité & Éclairage", "الكهرباء والإنارة"),
    (
        "CAT-VISS",
        "Visserie, Boulonnerie & Fixation",
        "البراغي والتثبيت",
    ),
    ("CAT-OUTIL", "Outillage & Équipement", "الأدوات والمعدات"),
    (
        "CAT-PEINT",
        "Peinture & Droguerie",
        "الدهانات والمواد الكيميائية",
    ),
    (
        "CAT-BATIM",
        "Quincaillerie de Bâtiment",
        "لوازم البناء والنجارة",
    ),
    ("CAT-JARDIN", "Jardinage & Arrosage", "لوازم الحدائق والري"),
];

/// Create a category with a unique code (Gerant/Admin).
pub struct CreateCategoryUseCase<'a, C>
where
    C: ?Sized,
{
    categories: &'a C,
}

impl<'a, C> CreateCategoryUseCase<'a, C>
where
    C: CategoryRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(categories: &'a C) -> Self {
        Self { categories }
    }

    /// Validate, reject duplicate codes, and persist the category.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: CreateCategoryInput,
    ) -> Result<Category, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        let code = input.code.trim().to_string();
        let name_fr = input.name_fr.trim().to_string();
        let name_ar = input.name_ar.trim().to_string();
        Category::validate(&code, &name_ar, &name_fr).map_err(ApplicationError::Validation)?;
        if let Some(parent_id) = input.parent_id {
            if self.categories.find_category_by_id(parent_id)?.is_none() {
                return Err(ApplicationError::NotFound("category".to_string()));
            }
        }
        let existing = self.categories.list_categories()?;
        if existing.iter().any(|category| category.code == code) {
            return Err(ApplicationError::Conflict(
                "CATEGORY_CODE_DUPLICATE".to_string(),
            ));
        }
        let category = Category {
            id: CategoryId::new(),
            code,
            name_ar,
            name_fr,
            parent_id: input.parent_id,
            is_active: true,
        };
        self.categories.create_category(&category)?;
        Ok(category)
    }
}

/// Update a category's mutable fields (Gerant/Admin; code immutable).
pub struct UpdateCategoryUseCase<'a, C>
where
    C: ?Sized,
{
    categories: &'a C,
}

impl<'a, C> UpdateCategoryUseCase<'a, C>
where
    C: CategoryRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(categories: &'a C) -> Self {
        Self { categories }
    }

    /// Apply name, parent, and activity changes atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: UpdateCategoryInput,
    ) -> Result<Category, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        let mut category = match self.categories.find_category_by_id(input.id)? {
            Some(found) => found,
            None => return Err(ApplicationError::NotFound("category".to_string())),
        };
        let name_fr = input.name_fr.trim().to_string();
        let name_ar = input.name_ar.trim().to_string();
        Category::validate(&category.code, &name_ar, &name_fr)
            .map_err(ApplicationError::Validation)?;
        if let Some(parent_id) = input.parent_id {
            if parent_id == input.id {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidCanonicalString(
                        "category cannot parent itself".to_string(),
                    ),
                ));
            }
            if self.categories.find_category_by_id(parent_id)?.is_none() {
                return Err(ApplicationError::NotFound("category".to_string()));
            }
        }
        category.name_fr = name_fr;
        category.name_ar = name_ar;
        category.parent_id = input.parent_id;
        category.is_active = input.is_active;
        self.categories.update_category(&category)?;
        Ok(category)
    }
}

/// List categories (all roles; inactive hidden unless requested).
pub struct ListCategoriesUseCase<'a, C>
where
    C: ?Sized,
{
    categories: &'a C,
}

impl<'a, C> ListCategoriesUseCase<'a, C>
where
    C: CategoryRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(categories: &'a C) -> Self {
        Self { categories }
    }

    /// Return categories ordered by the repository (code order in SQLite).
    pub fn execute(
        &self,
        session: &SessionContext,
        include_inactive: bool,
    ) -> Result<Vec<Category>, ApplicationError> {
        require(session, Permission::PosSell)?;
        let categories = self.categories.list_categories()?;
        if include_inactive {
            Ok(categories)
        } else {
            Ok(categories
                .into_iter()
                .filter(|category| category.is_active)
                .collect())
        }
    }
}

/// Idempotently provision the hardware starter taxonomy (Gerant/Admin).
pub struct SeedStarterCategoriesUseCase<'a, C>
where
    C: ?Sized,
{
    categories: &'a C,
}

impl<'a, C> SeedStarterCategoriesUseCase<'a, C>
where
    C: CategoryRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(categories: &'a C) -> Self {
        Self { categories }
    }

    /// No-op success when any category exists; otherwise create the seven
    /// starter departments in taxonomy order.
    pub fn execute(&self, session: &SessionContext) -> Result<Vec<Category>, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        let existing = self.categories.list_categories()?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        let mut created = Vec::with_capacity(STARTER_CATEGORIES.len());
        for (code, name_fr, name_ar) in STARTER_CATEGORIES {
            let category = Category {
                id: CategoryId::new(),
                code: code.to_string(),
                name_ar: name_ar.to_string(),
                name_fr: name_fr.to_string(),
                parent_id: None,
                is_active: true,
            };
            self.categories.create_category(&category)?;
            created.push(category);
        }
        Ok(created)
    }
}

/// Hard ceiling per import call: bounds transaction time for instant UI.
pub const MAX_IMPORT_ROWS: usize = 1000;

/// Duplicate-SKU policy for batch imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportDuplicatePolicy {
    /// Record the row in `skipped` and continue with valid rows.
    Skip,
    /// Abort the whole batch (rolls back) on the first duplicate.
    Error,
}

/// One raw batch row (application shape; the commands DTO maps onto this).
#[derive(Debug, Clone)]
pub struct ImportProductRow {
    /// Optional SKU; trimmed + uppercased, generated when absent.
    pub sku: Option<String>,
    /// Optional base-unit barcode; cleared when already assigned.
    pub barcode: Option<String>,
    /// Required French designation.
    pub name_fr: String,
    /// Arabic designation; falls back to `name_fr`.
    pub name_ar: Option<String>,
    /// Owning category UUID string; detached when unknown.
    pub category_id: Option<String>,
    /// Base-unit display names; `Pièce` / `قطعة` defaults.
    pub base_unit_name_fr: Option<String>,
    /// Base-unit display names; `Pièce` / `قطعة` defaults.
    pub base_unit_name_ar: Option<String>,
    /// Purchase centimes; zero when absent.
    pub purchase_price_centimes: Option<i64>,
    /// Sale centimes; must be non-negative.
    pub sale_price_centimes: i64,
    /// Opening quantity in base micros; provisioned when positive.
    pub initial_quantity_micro: Option<i64>,
}

/// Validated row: names trimmed, prices checked. Pure output of
/// [`validate_import_row`].
#[derive(Debug, Clone)]
pub struct ValidatedImportRow {
    /// Trimmed French designation (non-empty).
    pub name_fr: String,
    /// Trimmed Arabic designation (non-empty; `name_fr` fallback applied).
    pub name_ar: String,
    /// Trimmed barcode when supplied.
    pub barcode: Option<String>,
    /// Trimmed SKU when supplied.
    pub sku: Option<String>,
    /// Trimmed category id when supplied.
    pub category_id: Option<String>,
    /// Trimmed base-unit French name when supplied.
    pub base_unit_name_fr: Option<String>,
    /// Trimmed base-unit Arabic name when supplied.
    pub base_unit_name_ar: Option<String>,
    /// Purchase centimes (zero when absent).
    pub purchase_price_centimes: i64,
    /// Sale centimes (non-negative).
    pub sale_price_centimes: i64,
    /// Opening quantity in base micros (zero when absent).
    pub initial_quantity_micro: i64,
}

/// Batch input: rows plus destination warehouse and duplicate policy.
#[derive(Debug, Clone)]
pub struct ImportBatchInput {
    /// Rows to ingest (clamped to [`MAX_IMPORT_ROWS`]).
    pub rows: Vec<ImportProductRow>,
    /// Warehouse code (default `DEP-01`); falls back to first active depot.
    pub warehouse_code: Option<String>,
    /// Duplicate-SKU policy (default [`ImportDuplicatePolicy::Skip`]).
    pub on_duplicate: ImportDuplicatePolicy,
    /// Terminal id recorded as product origin.
    pub origin_device_id: String,
}

/// Per-row failure entry of the execution report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowError {
    /// Zero-based input row index.
    pub row_index: usize,
    /// Resolved (or supplied) SKU when known.
    pub sku: Option<String>,
    /// Row designation for operator display.
    pub name_fr: String,
    /// Stable machine-readable reason.
    pub error: String,
}

/// Structured execution report of one import batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportBatchReport {
    /// Rows ingested (after clamping).
    pub total_rows: usize,
    /// Rows fully created with base unit (and stock when requested).
    pub imported_count: usize,
    /// Rows skipped on duplicate SKU (`Skip` policy).
    pub skipped_count: usize,
    /// Per-row validation and skip entries, in row order.
    pub errors: Vec<ImportRowError>,
    /// Wall-clock execution time in milliseconds.
    pub duration_ms: u64,
}

/// Pure row validation: non-blank French name, non-negative sale price.
/// Returns the trimmed row or a report entry. Pure and side-effect free.
pub fn validate_import_row(
    index: usize,
    row: &ImportProductRow,
) -> Result<ValidatedImportRow, ImportRowError> {
    let name_fr = row.name_fr.trim().to_string();
    if name_fr.is_empty() {
        return Err(ImportRowError {
            row_index: index,
            sku: row.sku.clone(),
            name_fr: row.name_fr.clone(),
            error: "BLANK_NAME_FR".to_string(),
        });
    }
    if row.sale_price_centimes < 0 {
        return Err(ImportRowError {
            row_index: index,
            sku: row.sku.clone(),
            name_fr,
            error: "NEGATIVE_SALE_PRICE".to_string(),
        });
    }
    let blank_to_none = |value: &Option<String>| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|trimmed| !trimmed.is_empty())
            .map(str::to_string)
    };
    Ok(ValidatedImportRow {
        name_fr: name_fr.clone(),
        name_ar: match blank_to_none(&row.name_ar) {
            Some(name_ar) => name_ar,
            None => name_fr,
        },
        barcode: blank_to_none(&row.barcode),
        sku: blank_to_none(&row.sku),
        category_id: blank_to_none(&row.category_id),
        base_unit_name_fr: blank_to_none(&row.base_unit_name_fr),
        base_unit_name_ar: blank_to_none(&row.base_unit_name_ar),
        purchase_price_centimes: row.purchase_price_centimes.unwrap_or(0),
        sale_price_centimes: row.sale_price_centimes,
        initial_quantity_micro: row.initial_quantity_micro.unwrap_or(0),
    })
}

/// Atomic batch-import store: validates-then-writes products, base units,
/// and opening stock inside ONE `BEGIN IMMEDIATE` transaction.
pub trait ProductImportRepository: Send + Sync {
    /// Ingest validated rows atomically; per-row failures are reported,
    /// never thrown, except the `Error` duplicate policy which aborts.
    fn import_batch(
        &self,
        actor: &SessionContext,
        rows: Vec<(usize, ValidatedImportRow)>,
        warehouse_code: Option<String>,
        on_duplicate: ImportDuplicatePolicy,
        origin_device_id: &str,
    ) -> Result<ImportBatchReport, ApplicationError>;
}

pub struct ImportProductsUseCase<'a, R>
where
    R: ?Sized,
{
    store: &'a R,
}

impl<'a, R> ImportProductsUseCase<'a, R>
where
    R: ProductImportRepository + ?Sized,
{
    /// Wire the use case to its store.
    pub fn new(store: &'a R) -> Self {
        Self { store }
    }

    /// Validate rows (Gerant/Admin), clamp to [`MAX_IMPORT_ROWS`], and
    /// ingest atomically. Returns the structured execution report.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: ImportBatchInput,
    ) -> Result<ImportBatchReport, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        let started = std::time::Instant::now();
        let mut rows = Vec::new();
        let mut early_errors = Vec::new();
        for (index, row) in input.rows.iter().enumerate().take(MAX_IMPORT_ROWS) {
            match validate_import_row(index, row) {
                Ok(valid) => rows.push((index, valid)),
                Err(error) => early_errors.push(error),
            }
        }
        let total_rows = rows.len() + early_errors.len();
        let mut report = self.store.import_batch(
            session,
            rows,
            input.warehouse_code,
            input.on_duplicate,
            &input.origin_device_id,
        )?;
        report.total_rows = total_rows;
        let mut merged = early_errors;
        merged.append(&mut report.errors);
        merged.sort_by_key(|error| error.row_index);
        report.errors = merged;
        report.duration_ms = started.elapsed().as_millis() as u64;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::KitComponentRow;
    use baraka_domain::catalog::{Category, CategoryId};
    use baraka_domain::role::Role;
    use baraka_domain::user::UserId;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::ports::{SessionContext, SessionToken};

    struct FakeCatalog {
        products: Mutex<HashMap<ProductId, (Product, Vec<ProductUnit>)>>,
        kit_items: Mutex<HashMap<ProductId, Vec<KitItem>>>,
        categories: Mutex<HashMap<CategoryId, Category>>,
    }

    impl FakeCatalog {
        fn new() -> Self {
            Self {
                products: Mutex::new(HashMap::new()),
                kit_items: Mutex::new(HashMap::new()),
                categories: Mutex::new(HashMap::new()),
            }
        }

        fn seed_product(&self, sku: &str, is_kit: bool, is_active: bool) -> (Product, ProductUnit) {
            let id = ProductId::new();
            let product = Product {
                id,
                sku: Sku::new(sku).expect("sku"),
                barcode: None,
                name_ar: "منتج".to_string(),
                name_fr: "Produit".to_string(),
                category_id: None,
                is_kit,
                is_active,
            };
            let unit = ProductUnit {
                id: UnitId::new(),
                product_id: id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: None,
                sale_price: Money::from_centimes(1_000),
                purchase_price: Money::from_centimes(800),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            };
            self.products
                .lock()
                .expect("products")
                .insert(id, (product.clone(), vec![unit.clone()]));
            (product, unit)
        }
    }

    impl ProductRepository for FakeCatalog {
        fn create_product(
            &self,
            product: &Product,
            units: &[ProductUnit],
            _origin_device_id: &str,
        ) -> Result<(), ApplicationError> {
            let mut products = self.products.lock().expect("products");
            if products.values().any(|(p, _)| p.sku == product.sku) {
                return Err(ApplicationError::Validation(DomainError::DuplicateSku));
            }
            products.insert(product.id, (product.clone(), units.to_vec()));
            Ok(())
        }

        fn update_product(
            &self,
            product: &Product,
            units: &[ProductUnit],
        ) -> Result<(), ApplicationError> {
            let mut products = self.products.lock().expect("products");
            match products.get_mut(&product.id) {
                Some(entry) => {
                    *entry = (product.clone(), units.to_vec());
                    Ok(())
                }
                None => Err(ApplicationError::NotFound("product".to_string())),
            }
        }

        fn find_by_id(
            &self,
            id: ProductId,
        ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError> {
            Ok(self.products.lock().expect("products").get(&id).cloned())
        }

        fn find_by_barcode(
            &self,
            barcode: &Barcode,
        ) -> Result<Option<(Product, ProductUnit)>, ApplicationError> {
            for (product, units) in self.products.lock().expect("products").values() {
                for unit in units {
                    if unit.barcode.as_ref() == Some(barcode) {
                        return Ok(Some((product.clone(), unit.clone())));
                    }
                }
            }
            Ok(None)
        }

        fn find_by_sku(
            &self,
            sku: &Sku,
        ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError> {
            for (product, units) in self.products.lock().expect("products").values() {
                if product.sku == *sku {
                    return Ok(Some((product.clone(), units.clone())));
                }
            }
            Ok(None)
        }

        fn define_kit(&self, kit_id: ProductId, items: &[KitItem]) -> Result<(), ApplicationError> {
            self.kit_items
                .lock()
                .expect("kits")
                .insert(kit_id, items.to_vec());
            Ok(())
        }

        fn get_kit_items(&self, kit_id: ProductId) -> Result<Vec<KitItem>, ApplicationError> {
            Ok(self
                .kit_items
                .lock()
                .expect("kits")
                .get(&kit_id)
                .cloned()
                .unwrap_or_default())
        }

        fn get_all_kit_edges(&self) -> Result<Vec<(ProductId, ProductId)>, ApplicationError> {
            let mut edges = Vec::new();
            for (kit, items) in self.kit_items.lock().expect("kits").iter() {
                for item in items {
                    edges.push((*kit, item.component_product_id));
                }
            }
            Ok(edges)
        }

        fn search(
            &self,
            query: &str,
            limit: usize,
        ) -> Result<Vec<(Product, Vec<ProductUnit>)>, ApplicationError> {
            let mut out = Vec::new();
            for (product, units) in self.products.lock().expect("products").values() {
                if product.sku.as_str().contains(query) {
                    out.push((product.clone(), units.clone()));
                }
                if out.len() >= limit {
                    break;
                }
            }
            Ok(out)
        }

        fn get_direct_kit_items(
            &self,
            kit_product_id: ProductId,
        ) -> Result<Vec<KitComponentRow>, ApplicationError> {
            Ok(self
                .kit_items
                .lock()
                .expect("kits")
                .get(&kit_product_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|item| KitComponentRow {
                    component_product_id: item.component_product_id,
                    unit_id: item.unit_id,
                    quantity_micro: item.quantity_micro.micros(),
                })
                .collect())
        }

        fn count_product_references(&self, _id: ProductId) -> Result<u64, ApplicationError> {
            Ok(0)
        }

        fn hard_delete_product(&self, id: ProductId) -> Result<(), ApplicationError> {
            let mut products = self.products.lock().expect("products");
            match products.remove(&id) {
                Some(_) => Ok(()),
                None => Err(ApplicationError::NotFound("product".to_string())),
            }
        }

        fn set_product_active(&self, id: ProductId, active: bool) -> Result<(), ApplicationError> {
            let mut products = self.products.lock().expect("products");
            match products.get_mut(&id) {
                Some((product, _)) => {
                    product.is_active = active;
                    Ok(())
                }
                None => Err(ApplicationError::NotFound("product".to_string())),
            }
        }
    }

    impl CategoryRepository for FakeCatalog {
        fn create_category(&self, category: &Category) -> Result<(), ApplicationError> {
            self.categories
                .lock()
                .expect("categories")
                .insert(category.id, category.clone());
            Ok(())
        }

        fn find_category_by_id(
            &self,
            id: CategoryId,
        ) -> Result<Option<Category>, ApplicationError> {
            Ok(self
                .categories
                .lock()
                .expect("categories")
                .get(&id)
                .cloned())
        }

        fn list_categories(&self) -> Result<Vec<Category>, ApplicationError> {
            Ok(self
                .categories
                .lock()
                .expect("categories")
                .values()
                .cloned()
                .collect())
        }

        fn update_category(&self, category: &Category) -> Result<(), ApplicationError> {
            let mut categories = self.categories.lock().expect("categories");
            match categories.get_mut(&category.id) {
                Some(entry) => {
                    *entry = category.clone();
                    Ok(())
                }
                None => Err(ApplicationError::NotFound("category".to_string())),
            }
        }
    }

    fn session_with(role: Role) -> SessionContext {
        SessionContext {
            token: SessionToken::new_random(),
            user_id: UserId::new(),
            role,
            expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
        }
    }

    fn base_input() -> Vec<NewUnitInput> {
        vec![NewUnitInput {
            unit_name_ar: "قطعة".to_string(),
            unit_name_fr: "Pièce".to_string(),
            factor_micro: 10_000,
            barcode: None,
            sale_price_centimes: 1_000,
            purchase_price_centimes: 800,
            wholesale_price_centimes: None,
            is_base_unit: true,
            is_default_selling: true,
        }]
    }

    #[test]
    fn vendeur_cannot_create_or_define_but_can_read() {
        let catalog = FakeCatalog::new();
        let vendeur = session_with(Role::Vendeur);
        let create = CreateProductUseCase::new(&catalog, &catalog);
        let input = CreateProductInput {
            sku: "VIS-8X40".to_string(),
            barcode: None,
            name_ar: "برغي".to_string(),
            name_fr: "Vis".to_string(),
            category_id: None,
            is_kit: false,
            units: base_input(),
        };
        assert!(matches!(
            create.execute(&vendeur, input, "TEST-TERMINAL"),
            Err(ApplicationError::Unauthorized)
        ));
        let define = DefineKitUseCase::new(&catalog);
        assert!(matches!(
            define.execute(&vendeur, ProductId::new(), Vec::new()),
            Err(ApplicationError::Unauthorized)
        ));
        let get = GetProductWithUnitsUseCase::new(&catalog);
        assert!(matches!(
            get.execute(&vendeur, ProductId::new()),
            Err(ApplicationError::NotFound(_))
        ));
    }

    #[test]
    fn gerant_creates_and_kit_cycle_rejected_end_to_end() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let create = CreateProductUseCase::new(&catalog, &catalog);
        let mk = |sku: &str, is_kit: bool| {
            create
                .execute(
                    &gerant,
                    CreateProductInput {
                        sku: sku.to_string(),
                        barcode: None,
                        name_ar: "قطعة".to_string(),
                        name_fr: "Piece".to_string(),
                        category_id: None,
                        is_kit,
                        units: base_input(),
                    },
                    "TEST-TERMINAL",
                )
                .expect("create")
        };
        let a = mk("KIT-A", true);
        let b = mk("LEAF-B", false);
        let define = DefineKitUseCase::new(&catalog);
        let unit_of = |id: ProductId| {
            catalog
                .find_by_id(id)
                .expect("lookup")
                .expect("product")
                .1
                .into_iter()
                .next()
                .expect("base unit")
                .id
        };
        // Self-reference rejected.
        let itself = define.execute(
            &gerant,
            a,
            vec![NewKitItemInput {
                component_id: a,
                unit_id: unit_of(a),
                quantity_micro: 10_000,
            }],
        );
        assert!(matches!(
            itself,
            Err(ApplicationError::Validation(
                DomainError::SelfReferencingKit
            ))
        ));
        // Valid edge accepted.
        define
            .execute(
                &gerant,
                a,
                vec![NewKitItemInput {
                    component_id: b,
                    unit_id: unit_of(b),
                    quantity_micro: 20_000,
                }],
            )
            .expect("valid kit");
        // Duplicate SKU rejected at the repository seam.
        let dup = create.execute(
            &gerant,
            CreateProductInput {
                sku: "KIT-A".to_string(),
                barcode: None,
                name_ar: "x".to_string(),
                name_fr: "y".to_string(),
                category_id: None,
                is_kit: false,
                units: base_input(),
            },
            "TEST-TERMINAL",
        );
        assert!(matches!(
            dup,
            Err(ApplicationError::Validation(DomainError::DuplicateSku))
        ));
    }

    #[test]
    fn inactive_component_is_rejected() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let kit = catalog.seed_product("KIT-X", true, true).0;
        let (dead, dead_unit) = catalog.seed_product("DEAD", false, false);
        let define = DefineKitUseCase::new(&catalog);
        let rejected = define.execute(
            &gerant,
            kit.id,
            vec![NewKitItemInput {
                component_id: dead.id,
                unit_id: dead_unit.id,
                quantity_micro: 10_000,
            }],
        );
        assert!(matches!(
            rejected,
            Err(ApplicationError::Validation(
                DomainError::InvalidKitComponent
            ))
        ));
    }

    #[test]
    fn gerant_updates_product_and_repeat_call_is_idempotent() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let (product, unit) = catalog.seed_product("VIS-8X40", false, true);
        let update = UpdateProductUseCase::new(&catalog, &catalog);
        let payload = || UpdateProductInput {
            id: product.id,
            name_ar: "برغي محدث".to_string(),
            name_fr: "Vis MAJ".to_string(),
            category_id: None,
            is_active: true,
            units: vec![UnitUpsert {
                id: Some(unit.id),
                unit: NewUnitInput {
                    sale_price_centimes: 1_500,
                    ..base_input().into_iter().next().expect("base")
                },
            }],
        };
        update.execute(&gerant, payload()).expect("first update");
        // Identical repeat: same state, no error (idempotent-safe).
        update.execute(&gerant, payload()).expect("repeat update");
        let (stored, units) = catalog
            .find_by_id(product.id)
            .expect("lookup")
            .expect("row");
        assert_eq!(stored.name_fr, "Vis MAJ");
        assert_eq!(stored.sku.as_str(), "VIS-8X40");
        assert!(stored.is_active);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].id, unit.id);
        assert_eq!(units[0].sale_price.centimes(), 1_500);
    }

    #[test]
    fn update_rejects_unknown_id_unauthorized_role_and_empty_names() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let (product, unit) = catalog.seed_product("VIS-8X40", false, true);
        let update = UpdateProductUseCase::new(&catalog, &catalog);
        let payload = |id: ProductId, name_fr: &str| UpdateProductInput {
            id,
            name_ar: "برغي".to_string(),
            name_fr: name_fr.to_string(),
            category_id: None,
            is_active: true,
            units: vec![UnitUpsert {
                id: Some(unit.id),
                unit: base_input().into_iter().next().expect("base"),
            }],
        };
        assert!(matches!(
            update.execute(&gerant, payload(ProductId::new(), "Vis")),
            Err(ApplicationError::NotFound(_))
        ));
        let vendeur = session_with(Role::Vendeur);
        assert!(matches!(
            update.execute(&vendeur, payload(product.id, "Vis")),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            update.execute(&gerant, payload(product.id, "   ")),
            Err(ApplicationError::Validation(_))
        ));
    }

    #[test]
    fn update_rejects_unknown_category_and_clears_on_none() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let (product, unit) = catalog.seed_product("VIS-8X40", false, true);
        let update = UpdateProductUseCase::new(&catalog, &catalog);
        let payload = |category_id: Option<CategoryId>| UpdateProductInput {
            id: product.id,
            name_ar: "برغي".to_string(),
            name_fr: "Vis".to_string(),
            category_id,
            is_active: true,
            units: vec![UnitUpsert {
                id: Some(unit.id),
                unit: base_input().into_iter().next().expect("base"),
            }],
        };
        assert!(matches!(
            update.execute(
                &gerant,
                payload(Some(CategoryId::from(uuid::Uuid::new_v4())))
            ),
            Err(ApplicationError::NotFound(_))
        ));
        update
            .execute(&gerant, payload(None))
            .expect("clear category");
        let (stored, _) = catalog
            .find_by_id(product.id)
            .expect("lookup")
            .expect("row");
        assert!(stored.category_id.is_none());
    }

    fn category_input(code: &str, parent_id: Option<CategoryId>) -> CreateCategoryInput {
        CreateCategoryInput {
            code: code.to_string(),
            name_fr: "Plomberie".to_string(),
            name_ar: "الترصيص".to_string(),
            parent_id,
        }
    }

    #[test]
    fn gerant_creates_category_and_duplicate_code_rejected() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let create = CreateCategoryUseCase::new(&catalog);
        let created = create
            .execute(&gerant, category_input("CAT-PLOMB", None))
            .expect("create");
        assert_eq!(created.code, "CAT-PLOMB");
        assert!(created.is_active);
        let dup = create.execute(&gerant, category_input("CAT-PLOMB", None));
        assert!(matches!(
            dup,
            Err(ApplicationError::Conflict(message)) if message == "CATEGORY_CODE_DUPLICATE"
        ));
    }

    #[test]
    fn category_create_rejects_blank_fields_unknown_parent_and_vendeur() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let create = CreateCategoryUseCase::new(&catalog);
        let blank = CreateCategoryInput {
            code: "   ".to_string(),
            name_fr: "x".to_string(),
            name_ar: "y".to_string(),
            parent_id: None,
        };
        assert!(matches!(
            create.execute(&gerant, blank),
            Err(ApplicationError::Validation(_))
        ));
        let orphan = category_input("CAT-X", Some(CategoryId::new()));
        assert!(matches!(
            create.execute(&gerant, orphan),
            Err(ApplicationError::NotFound(_))
        ));
        let vendeur = session_with(Role::Vendeur);
        assert!(matches!(
            create.execute(&vendeur, category_input("CAT-Y", None)),
            Err(ApplicationError::Unauthorized)
        ));
    }

    #[test]
    fn gerant_updates_category_names_parent_and_active_flag() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let create = CreateCategoryUseCase::new(&catalog);
        let parent = create
            .execute(&gerant, category_input("CAT-PLOMB", None))
            .expect("parent");
        let child = create
            .execute(&gerant, category_input("CAT-VISS", None))
            .expect("child");
        let update = UpdateCategoryUseCase::new(&catalog);
        let updated = update
            .execute(
                &gerant,
                UpdateCategoryInput {
                    id: child.id,
                    name_fr: "Visserie MAJ".to_string(),
                    name_ar: "البراغي".to_string(),
                    parent_id: Some(parent.id),
                    is_active: false,
                },
            )
            .expect("update");
        assert_eq!(updated.code, "CAT-VISS");
        assert_eq!(updated.name_fr, "Visserie MAJ");
        assert_eq!(updated.parent_id, Some(parent.id));
        assert!(!updated.is_active);
        // Unknown id, self-parent, and unauthorized role rejected.
        assert!(matches!(
            update.execute(
                &gerant,
                UpdateCategoryInput {
                    id: CategoryId::new(),
                    name_fr: "x".to_string(),
                    name_ar: "y".to_string(),
                    parent_id: None,
                    is_active: true,
                }
            ),
            Err(ApplicationError::NotFound(_))
        ));
        assert!(matches!(
            update.execute(
                &gerant,
                UpdateCategoryInput {
                    id: child.id,
                    name_fr: "x".to_string(),
                    name_ar: "y".to_string(),
                    parent_id: Some(child.id),
                    is_active: true,
                }
            ),
            Err(ApplicationError::Validation(_))
        ));
        let vendeur = session_with(Role::Vendeur);
        assert!(matches!(
            update.execute(
                &vendeur,
                UpdateCategoryInput {
                    id: child.id,
                    name_fr: "x".to_string(),
                    name_ar: "y".to_string(),
                    parent_id: None,
                    is_active: true,
                }
            ),
            Err(ApplicationError::Unauthorized)
        ));
    }

    #[test]
    fn category_list_filters_inactive_and_all_roles_may_read() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let create = CreateCategoryUseCase::new(&catalog);
        let active = create
            .execute(&gerant, category_input("CAT-A", None))
            .expect("active");
        let dormant = create
            .execute(&gerant, category_input("CAT-B", None))
            .expect("dormant");
        UpdateCategoryUseCase::new(&catalog)
            .execute(
                &gerant,
                UpdateCategoryInput {
                    id: dormant.id,
                    name_fr: dormant.name_fr.clone(),
                    name_ar: dormant.name_ar.clone(),
                    parent_id: None,
                    is_active: false,
                },
            )
            .expect("deactivate");
        let vendeur = session_with(Role::Vendeur);
        let list = ListCategoriesUseCase::new(&catalog);
        let visible = list.execute(&vendeur, false).expect("list");
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, active.id);
        let all = list.execute(&vendeur, true).expect("list all");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn seed_starter_is_idempotent_and_lists_seven_departments() {
        let catalog = FakeCatalog::new();
        let gerant = session_with(Role::Gerant);
        let seed = SeedStarterCategoriesUseCase::new(&catalog);
        let created = seed.execute(&gerant).expect("seed");
        assert_eq!(created.len(), 7);
        let codes: Vec<&str> = created.iter().map(|item| item.code.as_str()).collect();
        assert_eq!(
            codes,
            vec![
                "CAT-PLOMB",
                "CAT-ELEC",
                "CAT-VISS",
                "CAT-OUTIL",
                "CAT-PEINT",
                "CAT-BATIM",
                "CAT-JARDIN"
            ]
        );
        assert!(created.iter().all(|item| item.is_active));
        // Second run is a no-op returning the same rows.
        let again = seed.execute(&gerant).expect("reseed");
        assert_eq!(again.len(), 7);
        assert_eq!(catalog.list_categories().expect("rows").len(), 7);
        // Unauthorized role cannot seed.
        let vendeur = session_with(Role::Vendeur);
        assert!(matches!(
            seed.execute(&vendeur),
            Err(ApplicationError::Unauthorized)
        ));
    }

    fn import_row(name_fr: &str, sale_price_centimes: i64) -> ImportProductRow {
        ImportProductRow {
            sku: None,
            barcode: None,
            name_fr: name_fr.to_string(),
            name_ar: None,
            category_id: None,
            base_unit_name_fr: None,
            base_unit_name_ar: None,
            purchase_price_centimes: None,
            sale_price_centimes,
            initial_quantity_micro: None,
        }
    }

    #[test]
    fn import_row_validation_rejects_blank_names_and_negative_prices() {
        let valid = validate_import_row(0, &import_row("Vis Placo", 1_500)).expect("valid");
        assert_eq!(valid.name_fr, "Vis Placo");
        assert_eq!(valid.name_ar, "Vis Placo");
        assert_eq!(valid.purchase_price_centimes, 0);
        let blank = validate_import_row(3, &import_row("   ", 1_500)).expect_err("blank");
        assert_eq!(blank.row_index, 3);
        assert_eq!(blank.error, "BLANK_NAME_FR");
        let negative = validate_import_row(5, &import_row("Vis", -1)).expect_err("negative");
        assert_eq!(negative.row_index, 5);
        assert_eq!(negative.error, "NEGATIVE_SALE_PRICE");
    }

    struct FakeImportStore {
        seen: Mutex<Vec<(usize, ValidatedImportRow)>>,
    }

    impl ProductImportRepository for FakeImportStore {
        fn import_batch(
            &self,
            _actor: &SessionContext,
            rows: Vec<(usize, ValidatedImportRow)>,
            _warehouse_code: Option<String>,
            _on_duplicate: ImportDuplicatePolicy,
            _origin_device_id: &str,
        ) -> Result<ImportBatchReport, ApplicationError> {
            *self.seen.lock().expect("lock") = rows;
            Ok(ImportBatchReport {
                total_rows: 0,
                imported_count: 0,
                skipped_count: 0,
                errors: Vec::new(),
                duration_ms: 0,
            })
        }
    }

    #[test]
    fn import_use_case_clamps_to_max_rows_and_reports_total() {
        let store = FakeImportStore {
            seen: Mutex::new(Vec::new()),
        };
        let use_case = ImportProductsUseCase::new(&store);
        let gerant = session_with(Role::Gerant);
        let mut rows = Vec::new();
        for index in 0..(MAX_IMPORT_ROWS + 5) {
            rows.push(import_row(&format!("Produit {index}"), 1_000));
        }
        let report = use_case
            .execute(
                &gerant,
                ImportBatchInput {
                    rows,
                    warehouse_code: None,
                    on_duplicate: ImportDuplicatePolicy::Skip,
                    origin_device_id: "TERM-1".to_string(),
                },
            )
            .expect("import");
        assert_eq!(report.total_rows, MAX_IMPORT_ROWS);
        assert_eq!(store.seen.lock().expect("lock").len(), MAX_IMPORT_ROWS);
    }
}
