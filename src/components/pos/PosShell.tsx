/**
 * Sovereign POS cockpit shell: 56px topbar, 7/5 main grid, 56px key
 * footer, modal layer. Root owns the RTL/LTR direction.
 */

import type { ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { usePosKeyboard } from "@/hooks/use-pos-keyboard";
import { PosShellView } from "@/components/pos/PosShellView";
import { CheckoutModal } from "@/components/pos/modals/CheckoutModal";
import { CustomerSelectModal } from "@/components/pos/modals/CustomerSelectModal";
import { RecordPaymentModal } from "@/components/pos/modals/RecordPaymentModal";
import { DiscountModal } from "@/components/pos/modals/DiscountModal";
import { QuantityModal } from "@/components/pos/modals/QuantityModal";
import { OpenCashSessionModal } from "@/components/pos/modals/OpenCashSessionModal";
import { CloseCashSessionModal } from "@/components/pos/modals/CloseCashSessionModal";
import { RecordExpenseModal } from "@/components/pos/modals/RecordExpenseModal";
import { DocumentConversionModal } from "@/components/pos/modals/DocumentConversionModal";
import { BackupModal } from "@/components/pos/modals/BackupModal";
import { StoreIdentityModal } from "@/components/pos/modals/StoreIdentityModal";
import { ProductEditModal } from "@/components/pos/modals/ProductEditModal";
import { WarehouseTransferModal } from "@/components/pos/modals/WarehouseTransferModal";
import { ReturnModal } from "@/components/pos/modals/ReturnModal";
import { usePaymentModalStore } from "@/stores/payment-modal-store";
import { useConversionModalStore } from "@/stores/conversion-modal-store";
import { useReturnModalStore } from "@/stores/return-modal-store";
import { useBackupModalStore } from "@/stores/backup-modal-store";
import { useWarehouseTransferStore } from "@/stores/warehouse-transfer-store";
import { useStockAdjustModalStore } from "@/stores/stock-adjust-modal-store";
import { StockAdjustModal } from "@/components/pos/modals/StockAdjustModal";
import { useCartStore } from "@/stores/cart-store";
import { t } from "@/lib/i18n";

export function PosShell(): ReactElement {
  usePosKeyboard();
  const locale = usePosUiStore((state) => state.locale);
  const activeModal = usePosUiStore((state) => state.activeModal);
  const session = useSessionStore((state) => state.session);
  const manager =
    session !== null && (session.role === "GERANT" || session.role === "ADMIN");
  const admin = session !== null && session.role === "ADMIN";
  const banner = usePosUiStore((state) => state.banner);
  const setBanner = usePosUiStore((state) => state.setBanner);
  const openModal = usePosUiStore((state) => state.openModal);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const paymentTarget = usePaymentModalStore((state) => state.target);
  const clearPayment = usePaymentModalStore((state) => state.clearPayment);

  const closePayment = (): void => {
    const returnTo = paymentTarget?.returnTo ?? null;
    clearPayment();
    if (returnTo !== null) {
      openModal(returnTo);
    } else {
      closeModal();
    }
  };

  return (
    <div
      dir={locale === "ar-DZ" ? "rtl" : "ltr"}
      lang={locale === "ar-DZ" ? "ar" : "fr"}
      className="flex h-screen w-screen select-none flex-col overflow-hidden bg-slate-100 text-slate-900 dark:bg-slate-950 dark:text-slate-100"
    >
      {banner !== null && (
        <button
          type="button"
          onClick={() => {
            setBanner(null);
          }}
          className="shrink-0 bg-emerald-700 px-3 py-1 text-center text-xs font-bold text-white"
        >
          {banner}
        </button>
      )}
      <PosShellView />
      <button
        type="button"
        title={t(locale, "conversion.open")}
        aria-label={t(locale, "conversion.open")}
        onClick={() => {
          useConversionModalStore.getState().openModal("devis_to_bl");
          openModal("conversion");
        }}
        className="fixed bottom-20 end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-primary-600 px-4 text-sm font-black text-white shadow-2xl hover:bg-primary-500"
      >
        <span aria-hidden="true">⇄</span>
        {t(locale, "conversion.open")}
      </button>
      <button
        type="button"
        title={t(locale, "return.returnModalTitle")}
        aria-label={t(locale, "return.returnModalTitle")}
        onClick={() => {
          useReturnModalStore.getState().openModal();
          openModal("return");
        }}
        className="fixed bottom-36 end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-amber-600 px-4 text-sm font-black text-white shadow-2xl hover:bg-amber-500"
      >
        <span aria-hidden="true">↩</span>
        {t(locale, "return.returnModalTitle")}
      </button>
      {manager && (
        <button
          type="button"
          title={t(locale, "warehousesTitle")}
          aria-label={t(locale, "warehousesTitle")}
          onClick={() => {
            void useWarehouseTransferStore.getState().openModal();
            openModal("warehouse-transfer");
          }}
          className="fixed bottom-52 end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-sky-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-sky-600"
        >
          <span aria-hidden="true">⇄</span>
          {t(locale, "warehousesTitle")}
        </button>
      )}
      {manager && (
        <button
          type="button"
          title={t(locale, "stockAdjust.title")}
          aria-label={t(locale, "stockAdjust.title")}
          onClick={() => {
            useStockAdjustModalStore.getState().openModal();
            openModal("stock-adjust");
          }}
          className="fixed bottom-[17rem] end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-slate-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-slate-600"
        >
          <span aria-hidden="true">⚖</span>
          {t(locale, "stockAdjust.title")}
        </button>
      )}
      {activeModal === "checkout" && <CheckoutModal />}
      {activeModal === "customer" && <CustomerSelectModal />}
      {activeModal === "conversion" && <DocumentConversionModal />}
      {activeModal === "edit-product" && <ProductEditModal />}
      {activeModal === "warehouse-transfer" && <WarehouseTransferModal />}
      {activeModal === "stock-adjust" && <StockAdjustModal />}
      {admin && (
        <button
          type="button"
          title={t(locale, "backups.title")}
          aria-label={t(locale, "backups.title")}
          onClick={() => {
            void useBackupModalStore.getState().openModal();
            openModal("backup");
          }}
          className="fixed bottom-[21rem] end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-emerald-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-emerald-600"
        >
          <span aria-hidden="true">🛡</span>
          {t(locale, "backups.title")}
        </button>
      )}
      {activeModal === "backup" && <BackupModal />}
      {admin && (
        <button
          type="button"
          title={t(locale, "settings.identity.title")}
          aria-label={t(locale, "settings.identity.title")}
          onClick={() => {
            openModal("identity");
          }}
          className="fixed bottom-[25rem] end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-indigo-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-indigo-600"
        >
          <span aria-hidden="true">🖨</span>
          {t(locale, "settings.identity.title")}
        </button>
      )}
      {activeModal === "identity" && <StoreIdentityModal />}
      {activeModal === "return" && <ReturnModal />}
      {activeModal === "payment" && paymentTarget !== null && (
        <RecordPaymentModal
          customerId={paymentTarget.customerId}
          customerName={paymentTarget.customerName}
          currentBalanceCentimes={paymentTarget.balanceCentimes}
          isOpen
          onClose={closePayment}
          onSuccess={(newBalance) => {
            const customer = useCartStore.getState().customer;
            if (customer.kind === "named" && customer.id === paymentTarget.customerId) {
              useCartStore.getState().setCustomer({
                ...customer,
                balanceCentimes: newBalance,
              });
            }
          }}
        />
      )}
      {activeModal === "discount" && <DiscountModal />}
      {activeModal === "quantity" && <QuantityModal />}
      {activeModal === "cash-open" && <OpenCashSessionModal />}
      {activeModal === "cash-close" && <CloseCashSessionModal />}
      {activeModal === "cash-expense" && <RecordExpenseModal />}
    </div>
  );
}
