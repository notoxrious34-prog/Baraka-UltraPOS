//! Catalog commands: products, kits, categories, import.
use super::{
    error_code, parse_unit_dto, parse_upsert_unit_dto, parse_wire_id, parse_wire_int,
    require_session, to_response, AppState,
};
use baraka_application::errors::ApplicationError;
use baraka_application::use_cases::catalog::{
    CreateCategoryInput, CreateCategoryUseCase, CreateProductInput, CreateProductUseCase,
    DefineKitUseCase, GetCatalogMetricsUseCase, GetProductWithUnitsUseCase, ImportBatchInput,
    ImportDuplicatePolicy, ImportProductRow, ImportProductsUseCase, ListCategoriesUseCase,
    NewKitItemInput, ProductDetail, SearchCatalogUseCase, SeedStarterCategoriesUseCase,
    UpdateCategoryInput, UpdateCategoryUseCase, UpdateProductInput, UpdateProductUseCase,
};
use baraka_application::use_cases::deletion::DeleteProductUseCase;
use baraka_domain::catalog::{Category, CategoryId, ProductId, UnitId};

use crate::dto::{
    CatalogMetricsDto, CatalogSearchRequestDto, CategoryResponseDto, CreateCategoryRequestDto,
    CreateProductDto, DefineKitDto, DeleteEntityRequestDto, DeleteEntityResultDto, GetProductDto,
    ImportProductsRequestDto, ImportProductsResponseDto, ImportRowErrorDto, ProductResponseDto,
    UpdateCategoryRequestDto, UpdateProductDto,
};

/// `catalog_create_product` command: atomic product + unit-set creation.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_create_product(
    state: tauri::State<'_, AppState>,
    request: CreateProductDto,
) -> Result<ProductResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut units = Vec::with_capacity(request.units.len());
    for unit in &request.units {
        units.push(parse_unit_dto(unit).map_err(|error| error_code(&error))?);
    }
    let category_id = request
        .category_id
        .as_deref()
        .map(|text| parse_wire_id(text, "category_id").map(CategoryId::from))
        .transpose()
        .map_err(|error| error_code(&error))?;
    let create = CreateProductUseCase::new(state.catalog.as_ref(), state.categories.as_ref());
    let id = create
        .execute(
            &session,
            CreateProductInput {
                sku: request.sku,
                barcode: request.barcode,
                name_ar: request.name_ar,
                name_fr: request.name_fr,
                category_id,
                is_kit: request.is_kit,
                units,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    let detail = GetProductWithUnitsUseCase::new(state.catalog.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    Ok(to_response(detail))
}

/// `catalog_update_product` command: full-state product update (SKU immutable).
///
/// Same RBAC contract as creation (Gerant/Admin via the use case); full-state
/// payload echoed back as the fresh detail view, mirroring `catalog_create_product`.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_update_product(
    state: tauri::State<'_, AppState>,
    request: UpdateProductDto,
) -> Result<ProductResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id =
        parse_wire_id(&request.product_id, "product_id").map_err(|error| error_code(&error))?;
    let mut units = Vec::with_capacity(request.units.len());
    for unit in &request.units {
        units.push(parse_upsert_unit_dto(unit).map_err(|error| error_code(&error))?);
    }
    let category_id = request
        .category_id
        .as_deref()
        .map(|text| parse_wire_id(text, "category_id").map(CategoryId::from))
        .transpose()
        .map_err(|error| error_code(&error))?;
    UpdateProductUseCase::new(state.catalog.as_ref(), state.categories.as_ref())
        .execute(
            &session,
            UpdateProductInput {
                id: ProductId::from(id),
                name_ar: request.name_ar,
                name_fr: request.name_fr,
                category_id,
                is_active: request.is_active,
                units,
            },
        )
        .map_err(|error| error_code(&error))?;
    let detail = GetProductWithUnitsUseCase::new(state.catalog.as_ref())
        .execute(&session, ProductId::from(id))
        .map_err(|error| error_code(&error))?;
    Ok(to_response(detail))
}

/// `catalog_delete_product` command: dual-track safe deletion (ADR-092).
/// Hard-purges unreferenced products; deactivates historically referenced
/// ones. Catalog-manage roles only.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_delete_product(
    state: tauri::State<'_, AppState>,
    request: DeleteEntityRequestDto,
) -> Result<DeleteEntityResultDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.id, "id").map_err(|error| error_code(&error))?;
    let outcome = DeleteProductUseCase::new(state.catalog.as_ref())
        .execute(&session, ProductId::from(id), &request.reason)
        .map_err(|error| error_code(&error))?;
    Ok(DeleteEntityResultDto {
        id: outcome.id,
        mode: outcome.mode.to_string(),
        message: outcome.message.to_string(),
    })
}

/// `catalog_get_product` command: full product detail view.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_get_product(
    state: tauri::State<'_, AppState>,
    request: GetProductDto,
) -> Result<ProductResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id =
        parse_wire_id(&request.product_id, "product_id").map_err(|error| error_code(&error))?;
    let detail = GetProductWithUnitsUseCase::new(state.catalog.as_ref())
        .execute(&session, ProductId::from(id))
        .map_err(|error| error_code(&error))?;
    Ok(to_response(detail))
}

/// `catalog_search` command: barcode-exact and multilingual prefix search.
/// `include_inactive` admits `is_active = 0` rows (management views);
/// omitted or `false` excludes them (POS and sale paths).
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_search(
    state: tauri::State<'_, AppState>,
    request: CatalogSearchRequestDto,
) -> Result<Vec<ProductResponseDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let limit = request.limit.unwrap_or(20) as usize;
    let include_inactive = request.include_inactive.unwrap_or(false);
    let results = SearchCatalogUseCase::new(state.catalog.as_ref())
        .execute(&session, &request.query, limit, include_inactive)
        .map_err(|error| error_code(&error))?;
    let mut out = Vec::with_capacity(results.len());
    for (product, units) in results {
        let kit_items = if product.is_kit {
            state
                .catalog
                .get_kit_items(product.id)
                .map_err(|error| error_code(&error))?
        } else {
            Vec::new()
        };
        out.push(to_response(ProductDetail {
            product,
            units,
            kit_items,
        }));
    }
    Ok(out)
}

/// `catalog_get_metrics` command: database-level catalog aggregates
/// (total / in-stock / out-of-stock / inactive) for the management hub.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_get_metrics(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<CatalogMetricsDto, String> {
    let session = require_session(&state, &session_token)?;
    let metrics = GetCatalogMetricsUseCase::new(state.catalog.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(CatalogMetricsDto {
        total_products: metrics.total_products,
        in_stock: metrics.in_stock,
        out_of_stock: metrics.out_of_stock,
        inactive: metrics.inactive,
    })
}

/// `catalog_define_kit` command: replace a kit's component graph.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_define_kit(
    state: tauri::State<'_, AppState>,
    request: DefineKitDto,
) -> Result<(), String> {
    let session = require_session(&state, &request.session_token)?;
    let kit_id = parse_wire_id(&request.kit_id, "kit_id")
        .map(ProductId::from)
        .map_err(|error| error_code(&error))?;
    let mut items = Vec::with_capacity(request.items.len());
    for item in &request.items {
        items.push(NewKitItemInput {
            component_id: parse_wire_id(&item.component_id, "component_id")
                .map(ProductId::from)
                .map_err(|error| error_code(&error))?,
            unit_id: parse_wire_id(&item.unit_id, "unit_id")
                .map(UnitId::from)
                .map_err(|error| error_code(&error))?,
            quantity_micro: parse_wire_int(&item.quantity_micro, "quantity_micro")
                .map_err(|error| error_code(&error))?,
        });
    }
    DefineKitUseCase::new(state.catalog.as_ref())
        .execute(&session, kit_id, items)
        .map_err(|error| error_code(&error))
}

/// `catalog_import_products` command: atomic batch ingestion (Gerant/Admin).
///
/// Valid rows commit inside ONE `BEGIN IMMEDIATE` transaction; per-row
/// validation and duplicate failures are reported, never partial without
/// a report. `on_duplicate_sku: "error"` aborts the whole batch instead;
/// `"update"` overwrites existing products in place.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_import_products(
    state: tauri::State<'_, AppState>,
    request: ImportProductsRequestDto,
) -> Result<ImportProductsResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let on_duplicate = match request.on_duplicate_sku.as_deref() {
        Some("error") => ImportDuplicatePolicy::Error,
        Some("update") => ImportDuplicatePolicy::Update,
        _ => ImportDuplicatePolicy::Skip,
    };
    let rows = request
        .rows
        .into_iter()
        .map(|row| ImportProductRow {
            sku: row.sku,
            barcode: row.barcode,
            name_fr: row.name_fr,
            name_ar: row.name_ar,
            category_id: row.category_id,
            base_unit_name_fr: row.base_unit_name_fr,
            base_unit_name_ar: row.base_unit_name_ar,
            purchase_price_centimes: row.purchase_price_centimes,
            sale_price_centimes: row.sale_price_centimes,
            initial_quantity_micro: row.initial_quantity_micro,
        })
        .collect();
    let report = ImportProductsUseCase::new(state.imports.as_ref())
        .execute(
            &session,
            ImportBatchInput {
                rows,
                warehouse_code: request.warehouse_id,
                on_duplicate,
                origin_device_id: state.terminal_id.clone(),
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(ImportProductsResponseDto {
        total_rows: report.total_rows,
        imported_count: report.imported_count,
        updated_count: report.updated_count,
        skipped_count: report.skipped_count,
        errors: report
            .errors
            .into_iter()
            .map(|error| ImportRowErrorDto {
                row_index: error.row_index,
                sku: error.sku,
                name_fr: error.name_fr,
                error: error.error,
            })
            .collect(),
        duration_ms: report.duration_ms,
    })
}

/// Map a domain category to its wire view.
fn to_category_dto(category: &Category) -> CategoryResponseDto {
    CategoryResponseDto {
        id: category.id.to_string(),
        code: category.code.clone(),
        name_fr: category.name_fr.clone(),
        name_ar: category.name_ar.clone(),
        parent_id: category.parent_id.map(|id| id.to_string()),
        is_active: category.is_active,
    }
}

/// Parse an optional UUID wire id (`None` stays detached).
fn parse_optional_wire_id(
    text: Option<&str>,
    field: &'static str,
) -> Result<Option<CategoryId>, ApplicationError> {
    text.map(|value| parse_wire_id(value, field).map(CategoryId::from))
        .transpose()
}

/// `category_list` command: category taxonomy (all roles; inactive hidden
/// unless `include_inactive` is set).
#[tauri::command(rename_all = "snake_case")]
pub fn category_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
    include_inactive: Option<bool>,
) -> Result<Vec<CategoryResponseDto>, String> {
    let session = require_session(&state, &session_token)?;
    let categories = ListCategoriesUseCase::new(state.categories.as_ref())
        .execute(&session, include_inactive.unwrap_or(false))
        .map_err(|error| error_code(&error))?;
    Ok(categories.iter().map(to_category_dto).collect())
}

/// `category_create` command: unique-code category creation (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn category_create(
    state: tauri::State<'_, AppState>,
    request: CreateCategoryRequestDto,
) -> Result<CategoryResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let parent_id = parse_optional_wire_id(request.parent_id.as_deref(), "parent_id")
        .map_err(|error| error_code(&error))?;
    let category = CreateCategoryUseCase::new(state.categories.as_ref())
        .execute(
            &session,
            CreateCategoryInput {
                code: request.code,
                name_fr: request.name_fr,
                name_ar: request.name_ar,
                parent_id,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_category_dto(&category))
}

/// `category_update` command: mutable-field category update (Gerant/Admin;
/// code stays immutable).
#[tauri::command(rename_all = "snake_case")]
pub fn category_update(
    state: tauri::State<'_, AppState>,
    request: UpdateCategoryRequestDto,
) -> Result<CategoryResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.category_id, "category_id")
        .map(CategoryId::from)
        .map_err(|error| error_code(&error))?;
    let parent_id = parse_optional_wire_id(request.parent_id.as_deref(), "parent_id")
        .map_err(|error| error_code(&error))?;
    let category = UpdateCategoryUseCase::new(state.categories.as_ref())
        .execute(
            &session,
            UpdateCategoryInput {
                id,
                name_fr: request.name_fr,
                name_ar: request.name_ar,
                parent_id,
                is_active: request.is_active,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_category_dto(&category))
}

/// `category_seed_starter` command: idempotent Algerian hardware starter
/// taxonomy (Gerant/Admin; no-op returning existing rows when non-empty).
#[tauri::command(rename_all = "snake_case")]
pub fn category_seed_starter(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<CategoryResponseDto>, String> {
    let session = require_session(&state, &session_token)?;
    let categories = SeedStarterCategoriesUseCase::new(state.categories.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(categories.iter().map(to_category_dto).collect())
}
