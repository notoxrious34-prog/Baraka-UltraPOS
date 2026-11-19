//! Variant-matrix generation + 2D view (TASK-BRK-114, Wave 4).
//!
//! Generation fans a template product out into one product per
//! (`axis1`, `axis2`) cell: the ladder and tier prices are inherited,
//! unit-level barcodes are STRIPPED (never duplicated across cells),
//! price-break rungs ride along re-keyed, and the single
//! `generate_matrix` port call persists model + cells atomically.

use std::collections::HashMap;

use baraka_domain::catalog::{
    plan_variant_matrix, validate_unit_set, Barcode, CategoryId, Product, ProductId, Sku, UnitId,
    VariantMatrixSpec, VariantModel, VariantModelId,
};
use baraka_domain::errors::DomainError;
use baraka_domain::primitives::Money;
use baraka_domain::role::Permission;

use super::catalog::{require, require_any, MANAGE_PERMISSIONS};
use crate::errors::ApplicationError;
use crate::ports::{
    CategoryRepository, MatrixCellWrite, ProductRepository, SessionContext,
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
    struct FakeProducts {
        products: Mutex<HashMap<ProductId, (Product, Vec<ProductUnit>)>>,
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
            _: ProductId,
            _: &[baraka_domain::catalog::KitItem],
        ) -> Result<(), ApplicationError> {
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
    struct FakeMatrix {
        models: Mutex<HashMap<VariantModelId, VariantModel>>,
        writes: Mutex<Vec<(VariantModel, Vec<MatrixCellWrite>)>>,
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
            _: VariantModelId,
        ) -> Result<Vec<baraka_domain::catalog::VariantCell>, ApplicationError> {
            Ok(Vec::new())
        }

        fn matrix_view(
            &self,
            _: VariantModelId,
        ) -> Result<Vec<VariantMatrixViewRow>, ApplicationError> {
            Ok(Vec::new())
        }
    }

    fn session() -> SessionContext {
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
