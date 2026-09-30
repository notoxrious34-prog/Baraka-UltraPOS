//! Database trigger tests: append-only ledgers and finalized documents.

mod common;

use rusqlite::Connection;

const STAMP: &str = "2026-01-01T00:00:00Z";

fn seed_user(conn: &Connection) {
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, created_at, updated_at)
         VALUES ('u1', 'caissier1', 'hash', 'CASHIER', ?1, ?1)",
        [STAMP],
    )
    .expect("seed user");
}

fn seed_warehouse(conn: &Connection) {
    conn.execute(
        "INSERT INTO warehouses (id, code, name, is_default, is_active)
         VALUES ('w1', 'MAIN', 'Principal', 1, 1)",
        [],
    )
    .expect("seed warehouse");
}

fn seed_customer(conn: &Connection) {
    conn.execute(
        "INSERT INTO customers
         (id, code, name, credit_policy, created_at, updated_at, revision, origin_device_id)
         VALUES ('c1', 'C-001', 'Client Test', 'NONE', ?1, ?1, 1, 'dev1')",
        [STAMP],
    )
    .expect("seed customer");
}

fn seed_product_unit(conn: &Connection) {
    conn.execute(
        "INSERT INTO products
         (id, sku, name_ar, name_fr, created_at, updated_at, revision, origin_device_id)
         VALUES ('p1', 'SKU-1', 'منتج', 'Produit', ?1, ?1, 1, 'dev1')",
        [STAMP],
    )
    .expect("seed product");
    conn.execute(
        "INSERT INTO product_units
         (id, product_id, unit_name_ar, unit_name_fr, factor_micro,
          sale_price_centimes, purchase_price_centimes, is_base_unit, is_default_selling)
         VALUES ('pu1', 'p1', 'قطعة', 'Piece', 10000, 10000, 8000, 1, 1)",
        [],
    )
    .expect("seed unit");
}

fn seed_ledger_row(conn: &Connection) {
    conn.execute(
        "INSERT INTO customer_ledger
         (id, sequence_no, customer_id, entry_type, amount_centimes,
          running_balance_centimes, created_by, created_at)
         VALUES ('cl1', 1, 'c1', 'DEBT', 5000, 5000, 'u1', ?1)",
        [STAMP],
    )
    .expect("seed ledger");
}

fn seed_movement_row(conn: &Connection) {
    conn.execute(
        "INSERT INTO stock_movements
         (id, sequence_no, warehouse_id, product_id, movement_type,
          quantity_micro, created_by, created_at)
         VALUES ('sm1', 1, 'w1', 'p1', 'IN', 10000, 'u1', ?1)",
        [STAMP],
    )
    .expect("seed movement");
}

fn seed_document(conn: &Connection, id: &str, number: &str, status: &str) {
    conn.execute(
        "INSERT INTO documents
         (id, number, fiscal_year, type, party_type, party_id, warehouse_id,
          document_language, subtotal_ht_centimes, vat_total_centimes,
          global_discount_centimes, timbre_centimes, rounding_centimes,
          ttc_centimes, paid_amount_centimes, change_amount_centimes,
          payment_method, status, created_at, created_by, revision, origin_device_id)
         VALUES (?1, ?2, 2026, 'SALE', 'CUSTOMER', 'c1', 'w1', 'FR',
                 10000, 1900, 0, 0, 0, 11900, 11900, 0,
                 'ESPECES', ?3, '2026-01-01T00:00:00Z', 'u1', 1, 'dev1')",
        [id, number, status],
    )
    .expect("seed document");
}

fn seed_item(conn: &Connection) {
    conn.execute(
        "INSERT INTO document_items
         (id, document_id, product_id, unit_id, quantity_micro, quantity_base_micro,
          unit_price_centimes, discount_centimes, tva_rate_micro,
          line_ht_centimes, line_tva_centimes, line_ttc_centimes,
          cost_price_centimes, sort_order)
         VALUES ('di1', 'd1', 'p1', 'pu1', 10000, 10000,
                 10000, 0, 190000, 10000, 1900, 11900, 8000, 0)",
        [],
    )
    .expect("seed item");
}

fn is_forbidden(error: &rusqlite::Error) -> bool {
    format!("{error:?}").contains("MUTATION_FORBIDDEN")
}

#[test]
fn customer_ledger_rejects_update_and_delete() {
    let db = common::migrated_file_db();
    seed_user(&db.conn);
    seed_customer(&db.conn);
    seed_ledger_row(&db.conn);

    let update = db
        .conn
        .execute(
            "UPDATE customer_ledger SET amount_centimes = 1 WHERE id = 'cl1'",
            [],
        )
        .expect_err("ledger update must abort");
    assert!(is_forbidden(&update), "unexpected: {update:?}");

    let delete = db
        .conn
        .execute("DELETE FROM customer_ledger WHERE id = 'cl1'", [])
        .expect_err("ledger delete must abort");
    assert!(is_forbidden(&delete), "unexpected: {delete:?}");
}

#[test]
fn stock_movements_reject_update_and_delete() {
    let db = common::migrated_file_db();
    seed_user(&db.conn);
    seed_warehouse(&db.conn);
    seed_product_unit(&db.conn);
    seed_movement_row(&db.conn);

    let update = db
        .conn
        .execute(
            "UPDATE stock_movements SET quantity_micro = 1 WHERE id = 'sm1'",
            [],
        )
        .expect_err("movement update must abort");
    assert!(is_forbidden(&update), "unexpected: {update:?}");

    let delete = db
        .conn
        .execute("DELETE FROM stock_movements WHERE id = 'sm1'", [])
        .expect_err("movement delete must abort");
    assert!(is_forbidden(&delete), "unexpected: {delete:?}");
}

#[test]
fn finalized_document_rejects_update_but_draft_allows_it() {
    let db = common::migrated_file_db();
    seed_user(&db.conn);
    seed_warehouse(&db.conn);
    seed_customer(&db.conn);
    seed_document(&db.conn, "d1", "00001", "COMPLETED");
    seed_document(&db.conn, "d2", "00002", "DRAFT");

    let blocked = db
        .conn
        .execute(
            "UPDATE documents SET payment_method = 'CREDIT' WHERE id = 'd1'",
            [],
        )
        .expect_err("finalized update must abort");
    assert!(is_forbidden(&blocked), "unexpected: {blocked:?}");

    db.conn
        .execute(
            "UPDATE documents SET payment_method = 'CREDIT' WHERE id = 'd2'",
            [],
        )
        .expect("draft update stays mutable");
}

#[test]
fn finalized_document_items_reject_update_and_delete() {
    let db = common::migrated_file_db();
    seed_user(&db.conn);
    seed_warehouse(&db.conn);
    seed_customer(&db.conn);
    seed_product_unit(&db.conn);
    seed_document(&db.conn, "d1", "00001", "FINALIZED");
    seed_item(&db.conn);

    let update = db
        .conn
        .execute(
            "UPDATE document_items SET discount_centimes = 5 WHERE id = 'di1'",
            [],
        )
        .expect_err("finalized item update must abort");
    assert!(is_forbidden(&update), "unexpected: {update:?}");

    let delete = db
        .conn
        .execute("DELETE FROM document_items WHERE id = 'di1'", [])
        .expect_err("finalized item delete must abort");
    assert!(is_forbidden(&delete), "unexpected: {delete:?}");
}
