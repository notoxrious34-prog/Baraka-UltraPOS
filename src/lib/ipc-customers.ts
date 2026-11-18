/**
 * Customers bridge: directory, tiers, credit ledger, statements.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type {
  CreditOverview,
  CustomerDetail,
  CustomerSummary,
  LastPriceMemory,
  StatementEntry,
} from "@/types/pos";
import { call, type DeleteEntityResult } from "./ipc-kernel";

export interface NewCustomerWire {
  name: string;
  phone: string | null;
  address: string | null;
  credit_policy: "DISABLED" | "LIMITED" | "UNLIMITED";
  credit_limit_centimes: string | null;
  customer_tier?: "retail" | "artisan" | "wholesale" | "super_gros" | null;
}

export interface UpdateCustomerWire {
  customer_id: string;
  name?: string;
  phone?: string | null;
  address?: string | null;
  credit_policy?: "DISABLED" | "LIMITED" | "UNLIMITED";
  credit_limit_centimes?: string | null;
  customer_tier?: "retail" | "artisan" | "wholesale" | "super_gros" | null;
  is_active?: boolean;
}

/** Sparse customer update; resolves the refreshed full view. */
export async function ipcUpdateCustomer(
  sessionToken: string,
  input: UpdateCustomerWire,
): Promise<CustomerDetail> {
  return call<CustomerDetail>("customers_update", {
    request: { session_token: sessionToken, ...input },
  });
}

/** Guarded dual-track customer deletion: rejects non-zero balances, deactivates on history. */
export async function ipcDeleteCustomer(
  sessionToken: string,
  customerId: string,
  reason: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("customers_delete", {
    request: { session_token: sessionToken, id: customerId, reason },
  });
}

/** Register a customer; resolves the stored id. */
export async function ipcCreateCustomer(
  sessionToken: string,
  input: NewCustomerWire,
): Promise<string> {
  const wire = await call<CustomerDetail>("customers_create", {
    request: { session_token: sessionToken, ...input },
  });
  return wire.id;
}

/** Search the customer directory. */
export async function ipcSearchCustomers(
  sessionToken: string,
  query: string,
  limit: number,
): Promise<CustomerSummary[]> {
  interface SearchWire {
    customers: CustomerSummary[];
  }
  const wire = await call<SearchWire>("customers_search", {
    request: { session_token: sessionToken, query, limit },
  });
  return wire.customers;
}

/** Fetch one full customer view. */
export async function ipcGetCustomer(
  sessionToken: string,
  customerId: string,
): Promise<CustomerDetail> {
  return call<CustomerDetail>("customers_get", {
    request: { session_token: sessionToken, customer_id: customerId },
  });
}

/** Last transacted price recall (BRK-111, read-only). Resolves `null` when
 * the backend reports no cell (never sold to this customer). */
export async function ipcGetLastPrice(
  sessionToken: string,
  customerId: string,
  productId: string,
  unitId: string,
): Promise<LastPriceMemory | null> {
  const response = await call<{
    price_centimes: string | null;
    discount_per_unit_centimes: string | null;
    document_id: string | null;
    sold_at: string | null;
  }>("customers_last_price", {
    request: {
      session_token: sessionToken,
      customer_id: customerId,
      product_id: productId,
      unit_id: unitId,
    },
  });
  if (
    response.price_centimes === null ||
    response.discount_per_unit_centimes === null ||
    response.document_id === null ||
    response.sold_at === null
  ) {
    return null;
  }
  return {
    priceCentimes: response.price_centimes,
    discountPerUnitCentimes: response.discount_per_unit_centimes,
    documentId: response.document_id,
    soldAt: response.sold_at,
  };
}

/** List customers (active only). */
export async function ipcListCustomers(sessionToken: string): Promise<CustomerSummary[]> {
  interface ListWire {
    customers: CustomerSummary[];
  }
  const wire = await call<ListWire>("customers_list", {
    session_token: sessionToken,
  });
  return wire.customers;
}

/** Whole-book credit exposure (BRK-110): aggregates plus largest debtors. */
export async function ipcGetCreditOverview(
  sessionToken: string,
  topN?: number,
): Promise<CreditOverview> {
  return call<CreditOverview>("customers_credit_overview", {
    request: { session_token: sessionToken, top_n: topN ?? null },
  });
}

export interface CreditPaymentResult {
  ledger_entry_id: string;
  new_balance_centimes: string;
}

/** Record a customer credit payment; resolves the new balance. */
export async function ipcRecordCreditPayment(
  sessionToken: string,
  customerId: string,
  amountCentimes: string,
  reference: string | null,
  allowOverpayment: boolean,
): Promise<CreditPaymentResult> {
  return call<CreditPaymentResult>("credit_record_payment", {
    request: {
      session_token: sessionToken,
      customer_id: customerId,
      amount_centimes: amountCentimes,
      reference,
      allow_overpayment: allowOverpayment,
    },
  });
}

/** Fetch a customer statement in ledger order. */
export async function ipcGetStatement(
  sessionToken: string,
  customerId: string,
  fromDate: string | null,
  toDate: string | null,
): Promise<StatementEntry[]> {
  interface StatementWire {
    entries: StatementEntry[];
  }
  const wire = await call<StatementWire>("credit_get_statement", {
    request: {
      session_token: sessionToken,
      customer_id: customerId,
      from_date: fromDate,
      to_date: toDate,
    },
  });
  return wire.entries;
}
