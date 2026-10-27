//! SQLite document repository (drafts, reads, repair finalization).

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    ConvertBlToFactureCommand, ConvertBlToFactureResult, ConvertDevisToBlCommand,
    ConvertDevisToBlResult, DocumentRepository, DocumentSearchFilter, DocumentSummary,
    StoredPayment,
};
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::inventory::WarehouseId;
use baraka_domain::sales::{
    CustomerId, Document, DocumentId, DocumentItem, DocumentItemView, DocumentLanguage,
    DocumentStatus, DocumentType,
};
use baraka_domain::user::UserId;
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::persistence::transaction::run_transaction;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("document store failure".to_string())
}

/// Document persistence over a shared SQLite connection.
pub struct SqliteDocumentRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteDocumentRepository {
    /// Wrap the shared connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApplicationError> {
        self.db
            .lock()
            .map_err(|_| ApplicationError::Port("database lock poisoned".to_string()))
    }
}

fn parse_document_id(text: &str) -> Result<DocumentId, rusqlite::Error> {
    Uuid::parse_str(text)
        .map(DocumentId::from)
        .map_err(|_| rusqlite::Error::InvalidColumnName("document id".to_string()))
}

fn parse_user_id(text: &str) -> Result<UserId, rusqlite::Error> {
    Uuid::parse_str(text)
        .map(UserId::from)
        .map_err(|_| rusqlite::Error::InvalidColumnName("user id".to_string()))
}

fn parse_stamp(text: &str) -> Result<chrono::DateTime<Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(text)
        .map(|fixed| fixed.with_timezone(&Utc))
        .map_err(|_| rusqlite::Error::InvalidColumnName("timestamp".to_string()))
}

fn status_from_code(code: &str) -> Result<DocumentStatus, rusqlite::Error> {
    match code {
        "DRAFT" => Ok(DocumentStatus::Draft),
        "FINALIZED" => Ok(DocumentStatus::Finalized),
        "CANCELLED" => Ok(DocumentStatus::Cancelled),
        "PARTIALLY_RECEIVED" => Ok(DocumentStatus::PartiallyReceived),
        "FULLY_RECEIVED" => Ok(DocumentStatus::FullyReceived),
        "CONVERTED" => Ok(DocumentStatus::Converted),
        _ => Err(rusqlite::Error::InvalidColumnName("status".to_string())),
    }
}

fn map_document(row: &rusqlite::Row) -> Result<Document, rusqlite::Error> {
    let id_text: String = row.get(0)?;
    let number: String = row.get(1)?;
    let fiscal_year: i64 = row.get(2)?;
    let type_code: String = row.get(3)?;
    let party_id: Option<String> = row.get(4)?;
    let warehouse_text: String = row.get(5)?;
    let language_text: String = row.get(6)?;
    let subtotal: i64 = row.get(7)?;
    let vat: i64 = row.get(8)?;
    let discount: i64 = row.get(9)?;
    let timbre: i64 = row.get(10)?;
    let ttc: i64 = row.get(11)?;
    let paid: i64 = row.get(12)?;
    let change: i64 = row.get(13)?;
    let payment_method: String = row.get(14)?;
    let status_code: String = row.get(15)?;
    let created_at_text: String = row.get(16)?;
    let created_by_text: String = row.get(17)?;
    let notes: Option<String> = row.get(18)?;
    Ok(Document {
        id: parse_document_id(&id_text)?,
        number,
        fiscal_year: i32::try_from(fiscal_year)
            .map_err(|_| rusqlite::Error::InvalidColumnName("fiscal year".to_string()))?,
        doc_type: DocumentType::from_code(&type_code)
            .map_err(|_| rusqlite::Error::InvalidColumnName("document type".to_string()))?,
        status: status_from_code(&status_code)?,
        language: language_text
            .parse::<DocumentLanguage>()
            .map_err(|_| rusqlite::Error::InvalidColumnName("language".to_string()))?,
        warehouse_id: Uuid::parse_str(&warehouse_text)
            .map(WarehouseId::from)
            .map_err(|_| rusqlite::Error::InvalidColumnName("warehouse id".to_string()))?,
        party_id,
        subtotal_ht_centimes: subtotal,
        vat_total_centimes: vat,
        global_discount_centimes: discount,
        timbre_centimes: timbre,
        ttc_centimes: ttc,
        paid_amount_centimes: paid,
        change_amount_centimes: change,
        payment_method,
        notes,
        created_at: parse_stamp(&created_at_text)?,
        created_by: parse_user_id(&created_by_text)?,
    })
}

fn map_item(row: &rusqlite::Row) -> Result<DocumentItem, rusqlite::Error> {
    let id_text: String = row.get(0)?;
    let doc_text: String = row.get(1)?;
    let product_text: String = row.get(2)?;
    let unit_text: String = row.get(3)?;
    Ok(DocumentItem {
        id: Uuid::parse_str(&id_text)
            .map_err(|_| rusqlite::Error::InvalidColumnName("item id".to_string()))?,
        document_id: parse_document_id(&doc_text)?,
        product_id: Uuid::parse_str(&product_text)
            .map(ProductId::from)
            .map_err(|_| rusqlite::Error::InvalidColumnName("product id".to_string()))?,
        unit_id: Uuid::parse_str(&unit_text)
            .map(UnitId::from)
            .map_err(|_| rusqlite::Error::InvalidColumnName("unit id".to_string()))?,
        quantity_micro: row.get(4)?,
        quantity_base_micro: row.get(5)?,
        unit_price_centimes: row.get(6)?,
        discount_centimes: row.get(7)?,
        discount_per_unit_centimes: row.get(8)?,
        tva_rate_micro: row.get(9)?,
        line_ht_centimes: row.get(10)?,
        line_tva_centimes: row.get(11)?,
        line_ttc_centimes: row.get(12)?,
        cost_price_centimes: row.get(13)?,
        sort_order: row.get(14)?,
    })
}

/// Map one joined details row: frozen snapshot + live display designation.
/// Indices 0-14 mirror [`ITEM_COLUMNS`] (same shape as [`map_item`]);
/// names start at 15.
/// indices 15-19 carry the LEFT-JOINed designation (all NULL when the row
/// is missing).
fn map_item_view(row: &rusqlite::Row) -> Result<DocumentItemView, rusqlite::Error> {
    let item = map_item(row)?;
    let product_name_fr: Option<String> = row.get(15)?;
    Ok(DocumentItemView {
        product_missing: product_name_fr.is_none(),
        product_name_fr,
        product_name_ar: row.get(16)?,
        sku: row.get(17)?,
        unit_name_fr: row.get(18)?,
        unit_name_ar: row.get(19)?,
        item,
    })
}

fn map_payment(row: &rusqlite::Row) -> Result<StoredPayment, rusqlite::Error> {
    let id: String = row.get(0)?;
    let doc_text: String = row.get(1)?;
    let customer_text: Option<String> = row.get(2)?;
    Ok(StoredPayment {
        id,
        document_id: parse_document_id(&doc_text)?,
        customer_id: customer_text
            .map(|text| {
                Uuid::parse_str(&text)
                    .map(CustomerId::from)
                    .map_err(|_| rusqlite::Error::InvalidColumnName("customer id".to_string()))
            })
            .transpose()?,
        amount_centimes: row.get(3)?,
        method_code: row.get(4)?,
        cash_session_id: row.get(5)?,
    })
}

const DOCUMENT_COLUMNS: &str = "id, number, fiscal_year, type, party_id, warehouse_id, language, subtotal_ht_centimes, vat_total_centimes, global_discount_centimes, timbre_centimes, ttc_centimes, paid_amount_centimes, change_amount_centimes, payment_method, status, created_at, created_by, notes";
const ITEM_COLUMNS: &str = "id, document_id, product_id, unit_id, quantity_micro, quantity_base_micro, unit_price_centimes, discount_centimes, discount_per_unit_centimes, tva_rate_micro, line_ht_centimes, line_tva_centimes, line_ttc_centimes, cost_price_centimes, sort_order";

fn insert_document_tx(
    tx: &rusqlite::Transaction,
    document: &Document,
    stamp: &str,
    origin_device_id: &str,
) -> Result<(), rusqlite::Error> {
    tx.execute(
        "INSERT INTO documents
         (id, number, fiscal_year, type, party_type, party_id, warehouse_id,
          document_language, language, subtotal_ht_centimes, vat_total_centimes,
          global_discount_centimes, timbre_centimes, rounding_centimes,
          ttc_centimes, paid_amount_centimes, change_amount_centimes,
          payment_method, status, created_at, created_by, revision,
          origin_device_id, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, ?12, 0,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, 1, ?20, ?21)",
        rusqlite::params![
            document.id.to_string(),
            document.number,
            i64::from(document.fiscal_year),
            document.doc_type.code(),
            document.party_id.as_ref().map(|_| "CUSTOMER"),
            document.party_id,
            document.warehouse_id.to_string(),
            document.language.to_string(),
            document.subtotal_ht_centimes,
            document.vat_total_centimes,
            document.global_discount_centimes,
            document.timbre_centimes,
            document.ttc_centimes,
            document.paid_amount_centimes,
            document.change_amount_centimes,
            document.payment_method,
            document.status.code(),
            stamp,
            document.created_by.to_string(),
            origin_device_id,
            document.notes,
        ],
    )?;
    Ok(())
}

fn insert_items_tx(
    tx: &rusqlite::Transaction,
    items: &[DocumentItem],
) -> Result<(), rusqlite::Error> {
    for item in items {
        tx.execute(
            "INSERT INTO document_items
             (id, document_id, product_id, unit_id, quantity_micro,
              quantity_base_micro, unit_price_centimes, discount_centimes,
              discount_per_unit_centimes, tva_rate_micro, line_ht_centimes,
              line_tva_centimes, line_ttc_centimes, cost_price_centimes,
              sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                item.id.to_string(),
                item.document_id.to_string(),
                item.product_id.to_string(),
                item.unit_id.to_string(),
                item.quantity_micro,
                item.quantity_base_micro,
                item.unit_price_centimes,
                item.discount_centimes,
                item.discount_per_unit_centimes,
                item.tva_rate_micro,
                item.line_ht_centimes,
                item.line_tva_centimes,
                item.line_ttc_centimes,
                item.cost_price_centimes,
                item.sort_order,
            ],
        )?;
    }
    Ok(())
}

impl DocumentRepository for SqliteDocumentRepository {
    fn create_draft(
        &self,
        document: &Document,
        items: &[DocumentItem],
        origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            insert_document_tx(tx, document, &stamp, origin_device_id)?;
            insert_items_tx(tx, items)?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn cancel_draft(&self, id: DocumentId) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        let rows = guard
            .prepare("UPDATE documents SET status = 'CANCELLED' WHERE id = ?1 AND status = 'DRAFT'")
            .map_err(|_| store_failure())?
            .execute(rusqlite::params![id.to_string()])
            .map_err(|_| store_failure())?;
        if rows == 0 {
            return Err(ApplicationError::Conflict(
                "document is not a draft".to_string(),
            ));
        }
        Ok(())
    }

    fn get_by_id(&self, id: DocumentId) -> Result<Option<Document>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                &format!("SELECT {DOCUMENT_COLUMNS} FROM documents WHERE id = ?1"),
                [id.to_string()],
                map_document,
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn get_by_number(
        &self,
        doc_type_code: &str,
        fiscal_year: i32,
        number: &str,
    ) -> Result<Option<Document>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                &format!(
                    "SELECT {DOCUMENT_COLUMNS} FROM documents
                     WHERE type = ?1 AND fiscal_year = ?2 AND number = ?3"
                ),
                rusqlite::params![doc_type_code, i64::from(fiscal_year), number],
                map_document,
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn get_items(&self, id: DocumentId) -> Result<Vec<DocumentItem>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(&format!(
                "SELECT {ITEM_COLUMNS} FROM document_items WHERE document_id = ?1 ORDER BY sort_order"
            ))
            .map_err(|_| store_failure())?;
        let items = statement
            .query_map([id.to_string()], map_item)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(items)
    }

    fn get_items_with_names(
        &self,
        id: DocumentId,
    ) -> Result<Vec<DocumentItemView>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT di.id, di.document_id, di.product_id, di.unit_id, di.quantity_micro, \
                 di.quantity_base_micro, di.unit_price_centimes, di.discount_centimes, \
                 di.discount_per_unit_centimes, di.tva_rate_micro, di.line_ht_centimes, \
                 di.line_tva_centimes, di.line_ttc_centimes, di.cost_price_centimes, \
                 di.sort_order, \
                 p.name_fr, p.name_ar, p.sku, u.unit_name_fr, u.unit_name_ar \
                 FROM document_items di \
                 LEFT JOIN products p ON p.id = di.product_id \
                 LEFT JOIN product_units u ON u.id = di.unit_id \
                 WHERE di.document_id = ?1 ORDER BY di.sort_order",
            )
            .map_err(|_| store_failure())?;
        let items = statement
            .query_map([id.to_string()], map_item_view)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(items)
    }

    fn get_payments(&self, id: DocumentId) -> Result<Vec<StoredPayment>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT id, document_id, customer_id, amount_centimes, payment_method, cash_session_id
                 FROM payments WHERE document_id = ?1 ORDER BY rowid",
            )
            .map_err(|_| store_failure())?;
        let payments = statement
            .query_map([id.to_string()], map_payment)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(payments)
    }

    fn get_print_status(&self, id: DocumentId) -> Result<String, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT status FROM print_jobs WHERE document_id = ?1 ORDER BY rowid DESC LIMIT 1",
                [id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
            .map(|status| status.unwrap_or_else(|| "NONE".to_string()))
    }

    fn finalize_document(
        &self,
        document: &Document,
        items: &[DocumentItem],
        payments: &[StoredPayment],
        origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            insert_document_tx(tx, document, &stamp, origin_device_id)?;
            insert_items_tx(tx, items)?;
            for payment in payments {
                let sequence: i64 = tx.query_row(
                    "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM payments",
                    [],
                    |row| row.get(0),
                )?;
                tx.execute(
                    "INSERT INTO payments
                     (id, sequence_no, document_id, customer_id, amount_centimes,
                      payment_method, cash_session_id, created_by, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        payment.id,
                        sequence,
                        payment.document_id.to_string(),
                        payment.customer_id.map(|id| id.to_string()),
                        payment.amount_centimes,
                        payment.method_code,
                        payment.cash_session_id,
                        document.created_by.to_string(),
                        stamp,
                    ],
                )?;
            }
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn convert_devis_to_bl(
        &self,
        command: &ConvertDevisToBlCommand,
    ) -> Result<ConvertDevisToBlResult, ApplicationError> {
        let mut guard = self.lock()?;
        crate::adapters::sqlite_conversion_repository::convert_devis_to_bl_atomic(
            &mut guard, command,
        )
    }

    fn convert_bl_to_facture(
        &self,
        command: &ConvertBlToFactureCommand,
    ) -> Result<ConvertBlToFactureResult, ApplicationError> {
        let mut guard = self.lock()?;
        crate::adapters::sqlite_conversion_repository::convert_bl_to_facture_atomic(
            &mut guard, command,
        )
    }

    fn search_documents(
        &self,
        filter: &DocumentSearchFilter,
    ) -> Result<Vec<DocumentSummary>, ApplicationError> {
        // Defense in depth: the use case clamps already; never trust a raw limit.
        let limit = filter.limit.unwrap_or(20).clamp(1, 100);
        let pattern = filter.query.as_deref().map(|term| {
            let mut escaped = String::with_capacity(term.len() + 2);
            escaped.push('%');
            for grain in term.chars() {
                if matches!(grain, '%' | '_' | '\\') {
                    escaped.push('\\');
                }
                escaped.push(grain);
            }
            escaped.push('%');
            escaped
        });
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT d.id, d.number, d.type, d.status, d.party_id, c.name,
                        d.ttc_centimes,
                        (SELECT COUNT(*) FROM document_items WHERE document_id = d.id),
                        d.created_at
                 FROM documents AS d
                 LEFT JOIN customers AS c ON d.party_id = c.id
                 WHERE (?1 IS NULL OR d.type = ?1)
                   AND (?2 IS NULL OR d.status = ?2)
                   AND (?3 IS NULL OR d.number LIKE ?3 ESCAPE '\\'
                        OR c.name LIKE ?3 ESCAPE '\\')
                 ORDER BY d.created_at DESC
                 LIMIT ?4",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(
                rusqlite::params![filter.doc_type, filter.status, pattern, i64::from(limit),],
                |row| {
                    Ok(DocumentSummary {
                        id: row.get(0)?,
                        number: row.get(1)?,
                        doc_type: row.get(2)?,
                        status: row.get(3)?,
                        party_id: row.get(4)?,
                        party_name: row.get(5)?,
                        ttc_centimes: row.get(6)?,
                        items_count: row.get(7)?,
                        created_at: row.get(8)?,
                    })
                },
            )
            .map_err(|_| store_failure())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())
    }
}
