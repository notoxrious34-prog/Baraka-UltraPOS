//! Variant-matrix generation + 2D view (TASK-BRK-114, Wave 4).
//!
//! Generation fans a template product out into one product per
//! (`axis1`, `axis2`) cell: the ladder and tier prices are inherited,
//! unit-level barcodes are STRIPPED (never duplicated across cells),
//! price-break rungs ride along re-keyed, and the single
//! `generate_matrix` port call persists model + cells atomically.

use std::collections::HashMap;

use baraka_domain::catalog::{
    plan_serie_composition, plan_variant_matrix, validate_kit_edges, validate_unit_set, Barcode,
    CategoryId, KitItem, Product, ProductId, SerieLineRequest, Sku, UnitId, VariantMatrixSpec,
    VariantModel, VariantModelId,
};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::primitives::Money;
use baraka_domain::role::Permission;

use super::catalog::{require, require_any, MANAGE_PERMISSIONS};
use crate::errors::ApplicationError;
use crate::ports::{
    CategoryRepository, MatrixCellWrite, ProductRepository, SerieAvailabilityPort, SessionContext,
    VariantMatrixRepository, VariantMatrixViewRow,
};

/// Per-cell override supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellOverrideInput {
    /// First-axis value the override targets.
    pub axis1: String,
    /// Second-axis value the override targets.
    pub axis2: String,
    /// Replacement sale price in centimes (default-selling unit).
    pub sale_price_centimes: Option<i64>,
    /// Replacement product-level barcode (unique plan-wide).
    pub barcode: Option<String>,
}

/// Generation input: model naming + axes + template + overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateVariantMatrixInput {
    /// Arabic model name.
    pub name_ar: String,
    /// French model name.
    pub name_fr: String,
    /// First axis display name.
    pub axis1_name: String,
    /// Second axis display name.
    pub axis2_name: String,
    /// First-axis values.
    pub axis1_values: Vec<String>,
    /// Second-axis values.
    pub axis2_values: Vec<String>,
    /// Base SKU every cell SKU derives from.
    pub base_sku: String,
    /// Owning category, inherited by every generated product.
    pub category_id: Option<CategoryId>,
    /// Template product lending its ladder, tiers, and rungs.
    pub template_product_id: ProductId,
    /// Per-cell overrides.
    pub overrides: Vec<CellOverrideInput>,
}

/// Generation output: the model id plus every minted SKU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateVariantMatrixOutput {
    /// Persisted model id.
    pub model_id: VariantModelId,
    /// Minted cells.
    pub cell_count: usize,
    /// Every minted SKU in display order.
    pub skus: Vec<String>,
}

/// Generate a full variant matrix atomically.
pub struct GenerateVariantMatrixUseCase<'a, P, C, V>
where
    P: ?Sized,
    C: ?Sized,
    V: ?Sized,
{
    products: &'a P,
    categories: &'a C,
    matrix: &'a V,
}

impl<'a, P, C, V> GenerateVariantMatrixUseCase<'a, P, C, V>
where
    P: ProductRepository + ?Sized,
    C: CategoryRepository + ?Sized,
    V: VariantMatrixRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(products: &'a P, categories: &'a C, matrix: &'a V) -> Self {
        Self {
            products,
            categories,
            matrix,
        }
    }

    /// Plan, expand, and persist the matrix in one atomic write.
    /// `terminal_id` is TerminalContext-resolved (never client-supplied).
    pub fn execute(
        &self,
        session: &SessionContext,
        input: GenerateVariantMatrixInput,
        terminal_id: &str,
    ) -> Result<GenerateVariantMatrixOutput, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        if terminal_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_TERMINAL_ID".to_string()),
            ));
        }
        if let Some(category_id) = input.category_id {
            if self.categories.find_category_by_id(category_id)?.is_none() {
                return Err(ApplicationError::NotFound("category".to_string()));
            }
        }
        let (template, template_units) =
            match self.products.find_by_id(input.template_product_id)? {
                Some(found) => found,
                None => return Err(ApplicationError::NotFound("template_product".to_string())),
            };
        let axis1_refs: Vec<&str> = input.axis1_values.iter().map(String::as_str).collect();
        let axis2_refs: Vec<&str> = input.axis2_values.iter().map(String::as_str).collect();
        let override_refs: Vec<baraka_domain::catalog::CellSpecOverride<'_>> = input
            .overrides
            .iter()
            .map(|over| baraka_domain::catalog::CellSpecOverride {
                axis1: over.axis1.as_str(),
                axis2: over.axis2.as_str(),
                sale_price_centimes: over.sale_price_centimes,
                barcode: over.barcode.as_deref(),
            })
            .collect();
        let spec = VariantMatrixSpec {
            name_ar: &input.name_ar,
            name_fr: &input.name_fr,
            axis1_name: &input.axis1_name,
            axis2_name: &input.axis2_name,
            axis1_values: &axis1_refs,
            axis2_values: &axis2_refs,
            base_sku: &input.base_sku,
            category_id: input.category_id,
            overrides: &override_refs,
        };
        let plan = plan_variant_matrix(&spec).map_err(ApplicationError::Validation)?;
        // Inherited rungs are read once per template unit, then re-keyed.
        let mut template_breaks: HashMap<UnitId, Vec<baraka_domain::catalog::PriceBreak>> =
            HashMap::new();
        for unit in &template_units {
            let rungs = self
                .products
                .price_breaks(template.id, unit.id)
                .unwrap_or_default();
            template_breaks.insert(unit.id, rungs);
        }
        let mut writes = Vec::with_capacity(plan.cells.len());
        for cell in &plan.cells {
            let product_id = ProductId::new();
            let sku = Sku::new(&cell.sku).map_err(ApplicationError::Validation)?;
            let barcode = cell
                .barcode
                .as_deref()
                .map(Barcode::new)
                .transpose()
                .map_err(ApplicationError::Validation)?;
            let mut units = Vec::with_capacity(template_units.len());
            let mut new_ids: HashMap<UnitId, UnitId> = HashMap::new();
            for template_unit in &template_units {
                let new_id = UnitId::new();
                new_ids.insert(template_unit.id, new_id);
                let mut cloned = template_unit.clone();
                cloned.id = new_id;
                cloned.product_id = product_id;
                // Inherited unit barcodes are STRIPPED: duplicating them
                // across cells would violate the unit-barcode UNIQUE index.
                cloned.barcode = None;
                if cloned.is_default_selling {
                    if let Some(price) = cell.sale_price_centimes {
                        cloned.sale_price = Money::from_centimes(price);
                    }
                }
                units.push(cloned);
            }
            validate_unit_set(product_id, &units).map_err(ApplicationError::Validation)?;
            let mut breaks = Vec::new();
            for template_unit in &template_units {
                if let Some(rungs) = template_breaks.get(&template_unit.id) {
                    if !rungs.is_empty() {
                        breaks.push((new_ids[&template_unit.id], rungs.clone()));
                    }
                }
            }
            writes.push(MatrixCellWrite {
                product: Product {
                    id: product_id,
                    sku,
                    barcode,
                    name_ar: format!(
                        "{} {} {}",
                        plan.model.name_ar, cell.axis1_value, cell.axis2_value
                    ),
                    name_fr: format!(
                        "{} {} {}",
                        plan.model.name_fr, cell.axis1_value, cell.axis2_value
                    ),
                    category_id: input.category_id,
                    is_kit: false,
                    is_active: true,
                },
                units,
                breaks,
                axis1_value: cell.axis1_value.clone(),
                axis2_value: cell.axis2_value.clone(),
                axis1_sort: cell.axis1_sort,
                axis2_sort: cell.axis2_sort,
            });
        }
        let model = VariantModel {
            id: VariantModelId::new(),
            name_ar: plan.model.name_ar.clone(),
            name_fr: plan.model.name_fr.clone(),
            axis1_name: plan.model.axis1_name.clone(),
            axis2_name: plan.model.axis2_name.clone(),
            category_id: input.category_id,
            is_active: true,
        };
        self.matrix.generate_matrix(&model, &writes)?;
        Ok(GenerateVariantMatrixOutput {
            model_id: model.id,
            cell_count: writes.len(),
            skus: writes
                .iter()
                .map(|write| write.product.sku.as_str().to_string())
                .collect(),
        })
    }
}

/// Read a model's 2D grid (ONE query, display order).
pub struct GetModelMatrixViewUseCase<'a, V>
where
    V: ?Sized,
{
    matrix: &'a V,
}

impl<'a, V> GetModelMatrixViewUseCase<'a, V>
where
    V: VariantMatrixRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(matrix: &'a V) -> Self {
        Self { matrix }
    }

    /// Fetch the grid rows for POS (DSN-108) and receiving (DSN-109).
    pub fn execute(
        &self,
        session: &SessionContext,
        model_id: VariantModelId,
    ) -> Result<Vec<VariantMatrixViewRow>, ApplicationError> {
        require(session, Permission::PosSell)?;
        if self.matrix.find_variant_model(model_id)?.is_none() {
            return Err(ApplicationError::NotFound("variant_model".to_string()));
        }
        self.matrix.matrix_view(model_id)
    }
}

/// List variant models for the POS grid picker (DSN-108).
pub struct ListVariantModelsUseCase<'a, V>
where
    V: ?Sized,
{
    matrix: &'a V,
}

impl<'a, V> ListVariantModelsUseCase<'a, V>
where
    V: VariantMatrixRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(matrix: &'a V) -> Self {
        Self { matrix }
    }

    /// Fetch every model, ordered by Arabic name.
    pub fn execute(&self, session: &SessionContext) -> Result<Vec<VariantModel>, ApplicationError> {
        require(session, Permission::PosSell)?;
        self.matrix.list_models()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{KitComponentRow, SessionToken};
    use baraka_domain::catalog::{Category, ProductUnit};
    use baraka_domain::primitives::FactorMicro;
    use baraka_domain::user::UserId;
    use chrono::{Duration, Utc};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub(crate) struct FakeProducts {
        pub(crate) products: Mutex<HashMap<ProductId, (Product, Vec<ProductUnit>)>>,
        pub(crate) kits: Mutex<HashMap<ProductId, Vec<baraka_domain::catalog::KitItem>>>,
    }

    impl ProductRepository for FakeProducts {
        fn create_product(
            &self,
            product: &Product,
            units: &[ProductUnit],
            _origin_device_id: &str,
        ) -> Result<(), ApplicationError> {
            self.products
                .lock()
                .expect("products")
                .insert(product.id, (product.clone(), units.to_vec()));
            Ok(())
        }

        fn update_product(&self, _: &Product, _: &[ProductUnit]) -> Result<(), ApplicationError> {
            Ok(())
        }

        fn find_by_id(
            &self,
            id: ProductId,
        ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError> {
            Ok(self.products.lock().expect("products").get(&id).cloned())
        }

        fn find_by_barcode(
            &self,
            _: &Barcode,
        ) -> Result<Option<(Product, ProductUnit)>, ApplicationError> {
            Ok(None)
        }

        fn find_by_sku(
            &self,
            _: &Sku,
        ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError> {
            Ok(None)
        }

        fn define_kit(
            &self,
            kit_id: ProductId,
            items: &[baraka_domain::catalog::KitItem],
        ) -> Result<(), ApplicationError> {
            self.kits
                .lock()
                .expect("kits")
                .insert(kit_id, items.to_vec());
            Ok(())
        }

        fn get_kit_items(
            &self,
            _: ProductId,
        ) -> Result<Vec<baraka_domain::catalog::KitItem>, ApplicationError> {
            Ok(Vec::new())
        }

        fn get_all_kit_edges(&self) -> Result<Vec<(ProductId, ProductId)>, ApplicationError> {
            Ok(Vec::new())
        }

        fn search(
            &self,
            _: &str,
            _: usize,
            _: bool,
        ) -> Result<Vec<(Product, Vec<ProductUnit>)>, ApplicationError> {
            Ok(Vec::new())
        }

        fn get_metrics(&self) -> Result<crate::ports::CatalogMetrics, ApplicationError> {
            Ok(crate::ports::CatalogMetrics {
                total_products: 0,
                in_stock: 0,
                out_of_stock: 0,
                inactive: 0,
            })
        }

        fn get_direct_kit_items(
            &self,
            _: ProductId,
        ) -> Result<Vec<KitComponentRow>, ApplicationError> {
            Ok(Vec::new())
        }

        fn count_product_references(&self, _: ProductId) -> Result<u64, ApplicationError> {
            Ok(0)
        }

        fn hard_delete_product(&self, _: ProductId) -> Result<(), ApplicationError> {
            Ok(())
        }

        fn set_product_active(&self, _: ProductId, _: bool) -> Result<(), ApplicationError> {
            Ok(())
        }

        fn replace_price_breaks(
            &self,
            _: ProductId,
            _: UnitId,
            _: &[baraka_domain::catalog::PriceBreak],
        ) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeCategories;

    impl CategoryRepository for FakeCategories {
        fn create_category(&self, _: &Category) -> Result<(), ApplicationError> {
            Ok(())
        }

        fn find_category_by_id(&self, _: CategoryId) -> Result<Option<Category>, ApplicationError> {
            Ok(None)
        }

        fn list_categories(&self) -> Result<Vec<Category>, ApplicationError> {
            Ok(Vec::new())
        }

        fn update_category(&self, _: &Category) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    #[derive(Default)]
    pub(crate) struct FakeMatrix {
        pub(crate) models: Mutex<HashMap<VariantModelId, VariantModel>>,
        pub(crate) writes: Mutex<Vec<(VariantModel, Vec<MatrixCellWrite>)>>,
        pub(crate) cells: Mutex<HashMap<VariantModelId, Vec<baraka_domain::catalog::VariantCell>>>,
    }

    impl VariantMatrixRepository for FakeMatrix {
        fn generate_matrix(
            &self,
            model: &VariantModel,
            cells: &[MatrixCellWrite],
        ) -> Result<(), ApplicationError> {
            self.models
                .lock()
                .expect("models")
                .insert(model.id, model.clone());
            self.writes
                .lock()
                .expect("writes")
                .push((model.clone(), cells.to_vec()));
            Ok(())
        }

        fn find_variant_model(
            &self,
            id: VariantModelId,
        ) -> Result<Option<VariantModel>, ApplicationError> {
            Ok(self.models.lock().expect("models").get(&id).cloned())
        }

        fn matrix_cells(
            &self,
            model_id: VariantModelId,
        ) -> Result<Vec<baraka_domain::catalog::VariantCell>, ApplicationError> {
            Ok(self
                .cells
                .lock()
                .expect("cells")
                .get(&model_id)
                .cloned()
                .unwrap_or_default())
        }

        fn matrix_view(
            &self,
            _: VariantModelId,
        ) -> Result<Vec<VariantMatrixViewRow>, ApplicationError> {
            Ok(Vec::new())
        }

        fn list_models(&self) -> Result<Vec<VariantModel>, ApplicationError> {
            let mut models: Vec<VariantModel> = self
                .models
                .lock()
                .expect("models")
                .values()
                .cloned()
                .collect();
            models.sort_by(|left, right| left.name_ar.cmp(&right.name_ar));
            Ok(models)
        }
    }

    pub(crate) fn session() -> SessionContext {
        SessionContext {
            token: SessionToken::new_random(),
            user_id: UserId(Uuid::new_v4()),
            role: baraka_domain::role::Role::Admin,
            expires_at: Utc::now() + Duration::hours(1),
        }
    }

    fn template(pair: &FakeProducts) -> (Product, Vec<ProductUnit>) {
        let product = Product {
            id: ProductId::new(),
            sku: Sku::new("CTN-100").expect("sku"),
            barcode: None,
            name_ar: "سيرية قطن".to_string(),
            name_fr: "SERIE COTON".to_string(),
            category_id: None,
            is_kit: false,
            is_active: true,
        };
        let unit_id = UnitId::new();
        let unit = ProductUnit {
            id: unit_id,
            product_id: product.id,
            unit_name_ar: "قطعة".to_string(),
            unit_name_fr: "Pièce".to_string(),
            factor_micro: FactorMicro::from_micros(10_000),
            barcode: Some(Barcode::new("TEMPLATE-BARCODE").expect("barcode")),
            sale_price: Money::from_centimes(25_000),
            purchase_price: Money::from_centimes(15_000),
            wholesale_price: Some(Money::from_centimes(22_000)),
            super_gros_price: Some(Money::from_centimes(20_000)),
            is_base_unit: true,
            is_default_selling: true,
        };
        pair.products
            .lock()
            .expect("products")
            .insert(product.id, (product.clone(), vec![unit.clone()]));
        (product, vec![unit])
    }

    fn input(template_id: ProductId) -> GenerateVariantMatrixInput {
        GenerateVariantMatrixInput {
            name_ar: "سيرية قطن".to_string(),
            name_fr: "SERIE COTON".to_string(),
            axis1_name: "Taille".to_string(),
            axis2_name: "Couleur".to_string(),
            axis1_values: vec!["XL".to_string(), "M".to_string()],
            axis2_values: vec!["NOIR".to_string(), "BLANC".to_string()],
            base_sku: "CTN-100".to_string(),
            category_id: None,
            template_product_id: template_id,
            overrides: vec![CellOverrideInput {
                axis1: "XL".to_string(),
                axis2: "NOIR".to_string(),
                sale_price_centimes: Some(27_000),
                barcode: Some("CELL-XL-NOIR".to_string()),
            }],
        }
    }

    #[test]
    fn generation_fans_out_cells_with_inherited_ladder_and_stripped_barcodes() {
        let products = FakeProducts::default();
        let categories = FakeCategories;
        let matrix = FakeMatrix::default();
        let (template, template_units) = template(&products);
        let generate = GenerateVariantMatrixUseCase::new(&products, &categories, &matrix);
        let output = generate
            .execute(&session(), input(template.id), "TERM-01")
            .expect("generates");
        assert_eq!(output.cell_count, 4);
        assert_eq!(
            output.skus,
            vec![
                "CTN-100-M-BLANC",
                "CTN-100-M-NOIR",
                "CTN-100-XL-BLANC",
                "CTN-100-XL-NOIR"
            ]
        );
        let writes = matrix.writes.lock().expect("writes");
        assert_eq!(writes.len(), 1, "single atomic port call");
        let (_, cells) = &writes[0];
        assert_eq!(cells.len(), 4);
        for cell in cells {
            assert_eq!(
                cell.units.len(),
                template_units.len(),
                "ladder inherited per cell"
            );
            assert!(
                cell.units.iter().all(|unit| unit.barcode.is_none()),
                "unit barcodes stripped, never duplicated"
            );
            assert_eq!(
                cell.units
                    .iter()
                    .filter(|unit| unit.is_default_selling)
                    .count(),
                1
            );
        }
        let priced = cells
            .iter()
            .find(|cell| cell.axis1_value == "XL" && cell.axis2_value == "NOIR")
            .expect("override cell");
        assert_eq!(
            priced
                .units
                .iter()
                .find(|unit| unit.is_default_selling)
                .expect("default")
                .sale_price,
            Money::from_centimes(27_000)
        );
        assert_eq!(
            priced.product.barcode.as_ref().map(Barcode::as_str),
            Some("CELL-XL-NOIR")
        );
        let plain = cells
            .iter()
            .find(|cell| cell.axis1_value == "M")
            .expect("plain cell");
        assert_eq!(
            plain
                .units
                .iter()
                .find(|unit| unit.is_default_selling)
                .expect("default")
                .sale_price,
            Money::from_centimes(25_000),
            "untouched cells inherit the template price"
        );
    }

    #[test]
    fn generation_rejects_loudly_without_any_persist() {
        let products = FakeProducts::default();
        let categories = FakeCategories;
        let matrix = FakeMatrix::default();
        let (template, _) = template(&products);
        let generate = GenerateVariantMatrixUseCase::new(&products, &categories, &matrix);
        let mut bad = input(template.id);
        bad.axis1_values = vec!["XL".to_string(), "xl".to_string()];
        assert!(generate.execute(&session(), bad, "TERM-01").is_err());
        let mut missing = input(template.id);
        missing.template_product_id = ProductId::new();
        assert!(generate.execute(&session(), missing, "TERM-01").is_err());
        assert!(
            matrix.writes.lock().expect("writes").is_empty(),
            "no write on validation failure"
        );
    }

    #[test]
    fn view_requires_an_existing_model() {
        let matrix = FakeMatrix::default();
        let view = GetModelMatrixViewUseCase::new(&matrix);
        assert!(
            view.execute(&session(), VariantModelId::new()).is_err(),
            "unknown model"
        );
    }
}

/// One requested série line: a size plus pieces per single série.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerieLineInput {
    /// First-axis value (`M`, `44`, `Ø18` …).
    pub axis1_value: String,
    /// Pieces of this size per single série.
    pub pieces: u32,
}

/// Compose-série input: a kit header plus a size run on one color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeSerieInput {
    /// Série header product (must exist with `is_kit`).
    pub kit_id: ProductId,
    /// Source variant model.
    pub model_id: VariantModelId,
    /// Second-axis value the série runs on (`NOIR` …).
    pub axis2_value: String,
    /// Size run (`2M + 4L + 4XL + 2XXL` …).
    pub lines: Vec<SerieLineInput>,
    /// Warehouse for the availability precheck (`None` = structural only).
    pub warehouse_id: Option<WarehouseId>,
    /// How many séries the precheck covers (≥1 when a warehouse is given).
    pub serie_count: u32,
}

/// One reported shortfall: required vs on-hand in base micros.
/// Tolerant by design: reported in the output, never blocking composition
/// (a série template is defined BEFORE its components are received).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerieShortfall {
    /// Component product running short.
    pub product_id: ProductId,
    /// Component SKU for the operator message.
    pub sku: String,
    /// Required base micros (`qty_per_serie × serie_count`).
    pub required_micro: i64,
    /// On-hand base micros in the precheck warehouse.
    pub on_hand_micro: i64,
}

/// Compose-série output: persisted kit plus the availability report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeSerieOutput {
    /// Série header product.
    pub kit_id: ProductId,
    /// Persisted components.
    pub component_count: usize,
    /// Pieces per single série.
    pub total_pieces: u32,
    /// Shortfalls in the precheck warehouse (empty when sufficient
    /// or when no warehouse was given).
    pub shortfalls: Vec<SerieShortfall>,
}

/// Compose a size-run série onto a kit header (TASK-BRK-115).
///
/// Structural validation (header, model, cells, activity, default units,
/// acyclicity) rejects LOUDLY with nothing persisted; the stock precheck
/// only REPORTS shortfalls so a série can be defined before receiving.
pub struct ComposeSerieUseCase<'a, P, V, A>
where
    P: ?Sized,
    V: ?Sized,
    A: ?Sized,
{
    products: &'a P,
    matrix: &'a V,
    availability: &'a A,
}

impl<'a, P, V, A> ComposeSerieUseCase<'a, P, V, A>
where
    P: ProductRepository + ?Sized,
    V: VariantMatrixRepository + ?Sized,
    A: SerieAvailabilityPort + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(products: &'a P, matrix: &'a V, availability: &'a A) -> Self {
        Self {
            products,
            matrix,
            availability,
        }
    }

    /// Resolve the size run, precheck availability, persist the kit graph.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: ComposeSerieInput,
    ) -> Result<ComposeSerieOutput, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        let invalid = |name: &str| {
            ApplicationError::Validation(DomainError::InvalidCanonicalString(name.to_string()))
        };
        let (kit, _) = match self.products.find_by_id(input.kit_id)? {
            Some(found) => found,
            None => return Err(ApplicationError::NotFound("kit product".to_string())),
        };
        if !kit.is_kit {
            return Err(ApplicationError::Validation(
                DomainError::InvalidKitComponent,
            ));
        }
        if self.matrix.find_variant_model(input.model_id)?.is_none() {
            return Err(ApplicationError::NotFound("variant_model".to_string()));
        }
        let cells = self.matrix.matrix_cells(input.model_id)?;
        let roster: Vec<(String, String)> = cells
            .iter()
            .map(|cell| (cell.axis1_value.clone(), cell.axis2_value.clone()))
            .collect();
        let line_refs: Vec<SerieLineRequest<'_>> = input
            .lines
            .iter()
            .map(|line| SerieLineRequest {
                axis1_value: line.axis1_value.as_str(),
                pieces: line.pieces,
            })
            .collect();
        let plan = plan_serie_composition(&roster, &input.axis2_value, &line_refs)
            .map_err(ApplicationError::Validation)?;
        if input.warehouse_id.is_some() && input.serie_count == 0 {
            return Err(invalid("SERIE_COUNT_ZERO"));
        }
        let mut kit_items = Vec::with_capacity(plan.len());
        let mut components = Vec::with_capacity(plan.len());
        let mut total_pieces: u32 = 0;
        for planned in &plan {
            let cell = &cells[planned.cell_index];
            let (component, component_units) = match self.products.find_by_id(cell.product_id)? {
                Some(found) => found,
                None => {
                    return Err(ApplicationError::Validation(
                        DomainError::InvalidKitComponent,
                    ))
                }
            };
            if !component.is_active {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidKitComponent,
                ));
            }
            let default_unit = component_units
                .iter()
                .find(|unit| unit.is_default_selling)
                .ok_or_else(|| {
                    ApplicationError::Validation(DomainError::InvalidCanonicalString(
                        "SERIE_NO_DEFAULT_UNIT".to_string(),
                    ))
                })?;
            let quantity_micro = i64::from(planned.pieces)
                .checked_mul(default_unit.factor_micro.micros())
                .ok_or_else(|| invalid("SERIE_QTY_OVERFLOW"))?;
            total_pieces = total_pieces
                .checked_add(planned.pieces)
                .ok_or_else(|| invalid("SERIE_TOO_LARGE"))?;
            components.push(cell.product_id);
            kit_items.push(KitItem {
                kit_product_id: input.kit_id,
                component_product_id: cell.product_id,
                unit_id: default_unit.id,
                quantity_micro: baraka_domain::primitives::QtyMicro::from_micros(quantity_micro),
            });
        }
        let stored: Vec<(ProductId, ProductId)> = self
            .products
            .get_all_kit_edges()?
            .into_iter()
            .filter(|(from, _)| *from != input.kit_id)
            .collect();
        validate_kit_edges(input.kit_id, &components, &stored)
            .map_err(ApplicationError::Validation)?;
        let mut shortfalls = Vec::new();
        if let Some(warehouse_id) = input.warehouse_id {
            for item in &kit_items {
                let (component, _) = self
                    .products
                    .find_by_id(item.component_product_id)?
                    .expect("component re-read");
                let required = item
                    .quantity_micro
                    .micros()
                    .checked_mul(i64::from(input.serie_count))
                    .ok_or_else(|| invalid("SERIE_QTY_OVERFLOW"))?;
                let on_hand = self
                    .availability
                    .component_on_hand(item.component_product_id, warehouse_id)?;
                if on_hand < required {
                    shortfalls.push(SerieShortfall {
                        product_id: item.component_product_id,
                        sku: component.sku.as_str().to_string(),
                        required_micro: required,
                        on_hand_micro: on_hand,
                    });
                }
            }
        }
        self.products.define_kit(input.kit_id, &kit_items)?;
        Ok(ComposeSerieOutput {
            kit_id: input.kit_id,
            component_count: kit_items.len(),
            total_pieces,
            shortfalls,
        })
    }
}

#[cfg(test)]
mod serie_compose_tests {
    use super::tests::{session, FakeMatrix, FakeProducts};
    use super::*;
    use baraka_domain::catalog::{ProductUnit, VariantCellId};
    use baraka_domain::inventory::WarehouseId as WhId;
    use baraka_domain::primitives::FactorMicro;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeAvailability {
        levels: Mutex<HashMap<(ProductId, WhId), i64>>,
    }

    impl SerieAvailabilityPort for FakeAvailability {
        fn component_on_hand(
            &self,
            product_id: ProductId,
            warehouse_id: WhId,
        ) -> Result<i64, ApplicationError> {
            Ok(self
                .levels
                .lock()
                .expect("levels")
                .get(&(product_id, warehouse_id))
                .copied()
                .unwrap_or(0))
        }
    }

    fn cell_product(products: &FakeProducts, sku: &str) -> ProductId {
        let id = ProductId::new();
        let unit = ProductUnit {
            id: UnitId::new(),
            product_id: id,
            unit_name_ar: "قطعة".to_string(),
            unit_name_fr: "Pièce".to_string(),
            factor_micro: FactorMicro::from_micros(10_000),
            barcode: None,
            sale_price: Money::from_centimes(25_000),
            purchase_price: Money::from_centimes(15_000),
            wholesale_price: None,
            super_gros_price: None,
            is_base_unit: true,
            is_default_selling: true,
        };
        products.products.lock().expect("products").insert(
            id,
            (
                Product {
                    id,
                    sku: Sku::new(sku).expect("sku"),
                    barcode: None,
                    name_ar: sku.to_string(),
                    name_fr: sku.to_string(),
                    category_id: None,
                    is_kit: false,
                    is_active: true,
                },
                vec![unit],
            ),
        );
        id
    }

    fn kit_header(products: &FakeProducts) -> ProductId {
        let id = ProductId::new();
        products.products.lock().expect("products").insert(
            id,
            (
                Product {
                    id,
                    sku: Sku::new("SERIE-NOIR-2442").expect("sku"),
                    barcode: None,
                    name_ar: "سيرية نوار".to_string(),
                    name_fr: "SERIE NOIR".to_string(),
                    category_id: None,
                    is_kit: true,
                    is_active: true,
                },
                Vec::new(),
            ),
        );
        id
    }

    fn model_with_cells(matrix: &FakeMatrix, cells: &[(ProductId, &str, &str)]) -> VariantModelId {
        let model_id = VariantModelId::new();
        matrix.models.lock().expect("models").insert(
            model_id,
            VariantModel {
                id: model_id,
                name_ar: "سيرية".to_string(),
                name_fr: "SERIE".to_string(),
                axis1_name: "Taille".to_string(),
                axis2_name: "Couleur".to_string(),
                category_id: None,
                is_active: true,
            },
        );
        matrix.cells.lock().expect("cells").insert(
            model_id,
            cells
                .iter()
                .enumerate()
                .map(
                    |(index, (product_id, axis1, axis2))| baraka_domain::catalog::VariantCell {
                        id: VariantCellId::new(),
                        model_id,
                        product_id: *product_id,
                        axis1_value: (*axis1).to_string(),
                        axis2_value: (*axis2).to_string(),
                        axis1_sort: index as i64,
                        axis2_sort: 0,
                    },
                )
                .collect(),
        );
        model_id
    }

    fn compose_input(kit_id: ProductId, model_id: VariantModelId) -> ComposeSerieInput {
        ComposeSerieInput {
            kit_id,
            model_id,
            axis2_value: "NOIR".to_string(),
            lines: vec![
                SerieLineInput {
                    axis1_value: "M".to_string(),
                    pieces: 2,
                },
                SerieLineInput {
                    axis1_value: "L".to_string(),
                    pieces: 4,
                },
                SerieLineInput {
                    axis1_value: "XL".to_string(),
                    pieces: 4,
                },
                SerieLineInput {
                    axis1_value: "XXL".to_string(),
                    pieces: 2,
                },
            ],
            warehouse_id: None,
            serie_count: 1,
        }
    }

    #[test]
    fn compose_persists_kit_graph_and_reports_tolerant_shortfalls() {
        let products = FakeProducts::default();
        let matrix = FakeMatrix::default();
        let availability = FakeAvailability::default();
        let kit_id = kit_header(&products);
        let m = cell_product(&products, "CTN-100-M-NOIR");
        let l = cell_product(&products, "CTN-100-L-NOIR");
        let xl = cell_product(&products, "CTN-100-XL-NOIR");
        let xxl = cell_product(&products, "CTN-100-XXL-NOIR");
        let model_id = model_with_cells(
            &matrix,
            &[
                (m, "M", "NOIR"),
                (l, "L", "NOIR"),
                (xl, "XL", "NOIR"),
                (xxl, "XXL", "NOIR"),
            ],
        );
        let warehouse = WhId::new();
        // 2 séries: M needs 4×10_000=40_000 (has 1_000_000), L needs 80_000
        // (has 10_000 → short), XL/XXL unchecked lines below cover exactness.
        availability
            .levels
            .lock()
            .expect("levels")
            .insert((m, warehouse), 1_000_000);
        availability
            .levels
            .lock()
            .expect("levels")
            .insert((l, warehouse), 10_000);
        availability
            .levels
            .lock()
            .expect("levels")
            .insert((xl, warehouse), 80_000);
        availability
            .levels
            .lock()
            .expect("levels")
            .insert((xxl, warehouse), 40_000);
        let compose = ComposeSerieUseCase::new(&products, &matrix, &availability);
        let mut input = compose_input(kit_id, model_id);
        input.warehouse_id = Some(warehouse);
        input.serie_count = 2;
        let output = compose.execute(&session(), input).expect("composes");
        assert_eq!(output.component_count, 4);
        assert_eq!(output.total_pieces, 12);
        assert_eq!(output.shortfalls.len(), 1, "only L runs short");
        assert_eq!(output.shortfalls[0].product_id, l);
        assert_eq!(output.shortfalls[0].required_micro, 80_000);
        assert_eq!(output.shortfalls[0].on_hand_micro, 10_000);
        // Tolerant: the kit graph persisted DESPITE the shortfall.
        let stored = products.kits.lock().expect("kits");
        let items = stored.get(&kit_id).expect("kit persisted");
        assert_eq!(items.len(), 4);
        let m_item = items
            .iter()
            .find(|item| item.component_product_id == m)
            .expect("M");
        assert_eq!(m_item.quantity_micro.micros(), 20_000);
    }

    #[test]
    fn compose_rejects_structural_failures_with_nothing_persisted() {
        let products = FakeProducts::default();
        let matrix = FakeMatrix::default();
        let availability = FakeAvailability::default();
        let kit_id = kit_header(&products);
        let m = cell_product(&products, "CTN-100-M-NOIR");
        let model_id = model_with_cells(&matrix, &[(m, "M", "NOIR")]);
        let compose = ComposeSerieUseCase::new(&products, &matrix, &availability);
        // Unknown size.
        let mut bad = compose_input(kit_id, model_id);
        bad.lines.push(SerieLineInput {
            axis1_value: "XXXL".to_string(),
            pieces: 1,
        });
        assert!(compose.execute(&session(), bad).is_err());
        // Non-kit header.
        let bad = compose_input(m, model_id);
        assert!(compose.execute(&session(), bad).is_err());
        // Missing model.
        let bad = compose_input(kit_id, VariantModelId::new());
        assert!(compose.execute(&session(), bad).is_err());
        // Zero serie count with a warehouse.
        let mut bad = compose_input(kit_id, model_id);
        bad.warehouse_id = Some(WhId::new());
        bad.serie_count = 0;
        assert!(compose.execute(&session(), bad).is_err());
        assert!(
            products.kits.lock().expect("kits").is_empty(),
            "no kit graph on structural failure"
        );
    }
}
