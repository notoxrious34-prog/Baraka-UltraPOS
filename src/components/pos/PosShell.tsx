/**
 * Sovereign POS cockpit shell: 56px topbar, 7/5 main grid, 56px key
 * footer, modal layer. Root owns the RTL/LTR direction.
 */

import type { ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
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
import { StockAdjustModal } from "@/components/pos/modals/StockAdjustModal";
import { ParkedSalesModal } from "@/components/pos/ParkedSalesModal";
import { NoticePopup } from "@/components/pos/NoticePopup";
import { useCartStore } from "@/stores/cart-store";

export function PosShell(): ReactElement {
  usePosKeyboard();
  const locale = usePosUiStore((state) => state.locale);
  const activeModal = usePosUiStore((state) => state.activeModal);
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
      className="flex h-full w-full select-none flex-col overflow-hidden bg-slate-100 text-slate-900 dark:bg-slate-950 dark:text-slate-100"
    >
      <PosShellView />
      <NoticePopup />
      {activeModal === "checkout" && <CheckoutModal />}
      {activeModal === "customer" && <CustomerSelectModal />}
      {activeModal === "conversion" && <DocumentConversionModal />}
      {activeModal === "edit-product" && <ProductEditModal />}
      {activeModal === "warehouse-transfer" && <WarehouseTransferModal />}
      {activeModal === "stock-adjust" && <StockAdjustModal />}
      {activeModal === "backup" && <BackupModal />}
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
      {activeModal === "parked" && <ParkedSalesModal />}
    </div>
  );
}
