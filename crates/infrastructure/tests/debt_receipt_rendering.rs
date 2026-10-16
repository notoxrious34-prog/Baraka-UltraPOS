//! Debt-payment receipt rendering (TASK-BRK-060, render-only).
//!
//! Separate pipeline from the sales receipt path: [`DebtReceiptDocument`]
//! carries ledger-sourced display strings (no `document_id`, no
//! sales-specific totals). These tests prove the new path renders in both
//! languages, suppresses the reference line correctly, reaches both
//! fallback branches, and never panics on adversarial input.

use baraka_application::ports::ReceiptRendererPort;
use baraka_domain::printing::{DebtReceiptDocument, ReceiptLanguage, StoreIdentity};
use baraka_infrastructure::printing::rasterizer::{
    encode_gs_v0, render_debt_canvas, EscPosRasterizer, PaperWidth, PAPER_WIDTH_DOTS,
};

fn sample_debt_receipt(language: ReceiptLanguage) -> DebtReceiptDocument {
    DebtReceiptDocument {
        language,
        shop_name: "BARAKA QUINCAILLERIE".to_string(),
        receipt_number: "RECU-2026-000042".to_string(),
        created_at: "2026-10-06T10:00:00Z".to_string(),
        customer_name: "Mohamed Benali".to_string(),
        amount_text: "1500.00".to_string(),
        new_balance_text: "3500.00".to_string(),
        reference_text: Some("CHQ-77821".to_string()),
        store_identity: StoreIdentity::default(),
        footer_message: String::new(),
    }
}

#[test]
fn debt_receipt_renders_french_with_reference() {
    let receipt = sample_debt_receipt(ReceiptLanguage::French);
    let canvas = render_debt_canvas(&receipt, PaperWidth::Width80mm).expect("french debt canvas");
    assert_eq!(canvas.width(), PAPER_WIDTH_DOTS);
    let bytes = encode_gs_v0(&canvas);
    assert!(!bytes.is_empty(), "debt receipt must produce output bytes");
    // Same receipt through the trait method must produce identical bytes.
    let renderer = EscPosRasterizer;
    let via_trait = renderer
        .render_debt_receipt(&receipt)
        .expect("trait render_debt_receipt");
    assert_eq!(bytes, via_trait);
}

#[test]
fn debt_receipt_renders_arabic() {
    let mut receipt = sample_debt_receipt(ReceiptLanguage::Arabic);
    receipt.customer_name = "محمد بن علي".to_string();
    let canvas = render_debt_canvas(&receipt, PaperWidth::Width80mm).expect("arabic debt canvas");
    assert_eq!(canvas.width(), PAPER_WIDTH_DOTS);
    let bytes = encode_gs_v0(&canvas);
    assert!(
        !bytes.is_empty(),
        "arabic debt receipt must produce output bytes"
    );
}

#[test]
fn debt_receipt_none_reference_renders_without_reference_line() {
    let with_reference = sample_debt_receipt(ReceiptLanguage::French);
    let mut without_reference = with_reference.clone();
    without_reference.reference_text = None;
    let with_bytes = encode_gs_v0(
        &render_debt_canvas(&with_reference, PaperWidth::Width80mm).expect("with reference canvas"),
    );
    let without_bytes = encode_gs_v0(
        &render_debt_canvas(&without_reference, PaperWidth::Width80mm)
            .expect("without reference canvas"),
    );
    assert!(!without_bytes.is_empty());
    assert_ne!(
        with_bytes, without_bytes,
        "omitting the reference must change the raster output"
    );
    assert!(
        without_bytes.len() < with_bytes.len(),
        "omitted reference line must shorten the output"
    );
}

#[test]
fn debt_receipt_whitespace_reference_renders_without_reference_line() {
    let mut receipt = sample_debt_receipt(ReceiptLanguage::French);
    receipt.reference_text = Some("   ".to_string());
    let bytes = encode_gs_v0(
        &render_debt_canvas(&receipt, PaperWidth::Width80mm).expect("whitespace ref canvas"),
    );
    assert!(!bytes.is_empty());
    let mut no_reference = sample_debt_receipt(ReceiptLanguage::French);
    no_reference.reference_text = None;
    let expected = encode_gs_v0(
        &render_debt_canvas(&no_reference, PaperWidth::Width80mm).expect("none ref canvas"),
    );
    assert_eq!(
        bytes, expected,
        "whitespace-only reference must render exactly like None"
    );
}

#[test]
fn debt_receipt_empty_footer_uses_default_line() {
    let receipt = sample_debt_receipt(ReceiptLanguage::French);
    assert!(receipt.footer_message.is_empty());
    let bytes = encode_gs_v0(
        &render_debt_canvas(&receipt, PaperWidth::Width80mm).expect("default footer canvas"),
    );
    assert!(!bytes.is_empty());
}

#[test]
fn debt_receipt_empty_store_name_uses_shop_name_fallback() {
    let receipt = sample_debt_receipt(ReceiptLanguage::Arabic);
    assert!(receipt.store_identity.store_name.is_empty());
    let bytes = encode_gs_v0(
        &render_debt_canvas(&receipt, PaperWidth::Width80mm).expect("fallback header canvas"),
    );
    assert!(!bytes.is_empty());
}

#[test]
fn debt_receipt_long_customer_name_never_panics() {
    let mut receipt = sample_debt_receipt(ReceiptLanguage::French);
    receipt.customer_name = "Benali ".repeat(200);
    assert!(receipt.customer_name.len() >= 200);
    let canvas = render_debt_canvas(&receipt, PaperWidth::Width80mm).expect("long name canvas");
    assert_eq!(canvas.width(), PAPER_WIDTH_DOTS);
    let bytes = encode_gs_v0(&canvas);
    assert!(!bytes.is_empty());
}
