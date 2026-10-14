/**
 * Customer modal (F2): guest default, live debounced backend search,
 * select-via-detail, and role-gated inline creation.
 *
 * Credit profiles come from the live `customers_get` backend view;
 * the backend remains the authoritative limit enforcer at checkout.
 */

import { useEffect, useState, type ReactElement } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t, translateBackendError } from "@/lib/i18n";
import { parseMoneyToCentimes } from "@/lib/money";
import {
  ipcCreateCustomer,
  ipcGetCustomer,
  ipcSearchCustomers,
} from "@/lib/ipc";
import { usePaymentModalStore } from "@/stores/payment-modal-store";
import type { CustomerSummary, CustomerTier, NamedCustomer } from "@/types/pos";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import { CustomerTierBadge } from "@/components/pos/CustomerTierBadge";

const DEBOUNCE_MS = 150;

function canCreate(role: string): boolean {
  return role === "CAISSIER" || role === "GERANT" || role === "ADMIN";
}

function canManageCredit(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

export function CustomerSelectModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const setCustomer = useCartStore((state) => state.setCustomer);
  const session = useSessionStore((state) => state.session);

  const [query, setQuery] = useState("");
  const [tierFilter, setTierFilter] = useState<"all" | CustomerTier>("all");
  const [results, setResults] = useState<CustomerSummary[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [selecting, setSelecting] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [newTier, setNewTier] = useState<CustomerTier>("retail");
  const [policy, setPolicy] = useState<"LIMITED" | "UNLIMITED" | "DISABLED">("DISABLED");
  const [limitText, setLimitText] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const role = session !== null ? session.role : "";
  const token = session !== null ? session.token : null;
  const openModal = usePosUiStore((state) => state.openModal);
  const openPayment = usePaymentModalStore((state) => state.openPayment);
  const [payingId, setPayingId] = useState<string | null>(null);

  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed === "" || token === null) {
      setResults([]);
      setSearching(false);
      return;
    }
    setSearching(true);
    const timer = window.setTimeout(() => {
      ipcSearchCustomers(token, trimmed, 20)
        .then((customers) => {
          setResults(customers);
          setSearchError(null);
        })
        .catch((failure: unknown) => {
          setSearchError(
            translateBackendError(
              typeof failure === "string" ? failure : "INTERNAL_ERROR",
              usePosUiStore.getState().locale,
            ),
          );
          setResults([]);
        })
        .finally(() => {
          setSearching(false);
        });
    }, DEBOUNCE_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [query, token]);

  const selectGuest = (): void => {
    setCustomer({ kind: "guest" });
    closeModal();
  };

  const selectCustomer = (id: string): void => {
    if (token === null) {
      return;
    }
    setSelecting(true);
    setSearchError(null);
    ipcGetCustomer(token, id)
      .then((detail) => {
        const customer: NamedCustomer = {
          kind: "named",
          id: detail.id,
          name: detail.name,
          policy: detail.credit_policy,
          limitCentimes: detail.credit_limit_centimes,
          balanceCentimes: detail.current_balance_centimes,
          tier: detail.customer_tier,
          source: "backend",
        };
        setCustomer(customer);
        closeModal();
      })
      .catch((failure: unknown) => {
        setSearchError(
          translateBackendError(
            typeof failure === "string" ? failure : "INTERNAL_ERROR",
            usePosUiStore.getState().locale,
          ),
        );
      })
      .finally(() => {
        setSelecting(false);
      });
  };

  const openVersementFor = (id: string): void => {
    if (token === null || payingId !== null) {
      return;
    }
    setPayingId(id);
    setSearchError(null);
    ipcGetCustomer(token, id)
      .then((detail) => {
        openPayment({
          customerId: detail.id,
          customerName: detail.name,
          balanceCentimes: detail.current_balance_centimes,
          returnTo: "customer",
        });
        openModal("payment");
      })
      .catch((failure: unknown) => {
        setSearchError(
          translateBackendError(
            typeof failure === "string" ? failure : "INTERNAL_ERROR",
            usePosUiStore.getState().locale,
          ),
        );
      })
      .finally(() => {
        setPayingId(null);
      });
  };

  const createAndSelect = (): void => {
    if (token === null) {
      return;
    }
    let limit: string | null = null;
    if (policy === "LIMITED") {
      if (!canManageCredit(role)) {
        setFormError(translateBackendError("UNAUTHORIZED", locale));
        return;
      }
      if (limitText.trim() === "") {
        setFormError(translateBackendError("INVALID_CREDIT_LIMIT", locale));
        return;
      }
      try {
        limit = parseMoneyToCentimes(limitText);
      } catch {
        setFormError(translateBackendError("INVALID_CREDIT_LIMIT", locale));
        return;
      }
    }
    setCreating(true);
    setFormError(null);
    ipcCreateCustomer(token, {
      name: name.trim(),
      phone: phone.trim() === "" ? null : phone.trim(),
      address: null,
      credit_policy: policy,
      credit_limit_centimes: limit,
      customer_tier: newTier,
    })
      .then((id) => {
        selectCustomer(id);
      })
      .catch((failure: unknown) => {
        setFormError(
          translateBackendError(
            typeof failure === "string" ? failure : "INTERNAL_ERROR",
            usePosUiStore.getState().locale,
          ),
        );
      })
      .finally(() => {
        setCreating(false);
      });
  };

  const inputClass =
    "h-11 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";

  const visible =
    tierFilter === "all" ? results : results.filter((entry) => entry.customer_tier === tierFilter);

  const tierOptions: Array<{ id: CustomerTier; label: string }> = [
    { id: "retail", label: t(locale, "tier_retail") },
    { id: "artisan", label: t(locale, "tier_artisan") },
    { id: "wholesale", label: t(locale, "tier_wholesale") },
  ];

  return (
    <ModalShell title={t(locale, "customer.title")} onClose={closeModal}>
      <div className="flex flex-col gap-3">
        <button
          type="button"
          onClick={selectGuest}
          className="h-11 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700"
        >
          {t(locale, "customer.guestUse")}
        </button>

        <input
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
          placeholder={t(locale, "customer.searchPlaceholder")}
          autoComplete="off"
          className={inputClass}
        />

        <div className="flex gap-2">
          {(
            [
              { id: "all", label: t(locale, "customer.filterAll") },
              { id: "artisan", label: t(locale, "tier_artisan") },
              { id: "wholesale", label: t(locale, "tier_wholesale") },
            ] as const
          ).map((option) => (
            <button
              key={option.id}
              type="button"
              onClick={() => {
                setTierFilter(option.id);
              }}
              className={`h-9 flex-1 rounded-lg px-2 text-xs font-black ${
                tierFilter === option.id
                  ? "bg-primary-600 text-white"
                  : "bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300"
              }`}
            >
              {option.label}
            </button>
          ))}
        </div>

        {searching && (
          <p className="text-center text-xs font-bold text-slate-500">
            {t(locale, "customer.searching")}
          </p>
        )}
        {searchError !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {searchError}
          </p>
        )}
        {!searching && searchError === null && query.trim() !== "" && results.length === 0 && (
          <p className="text-center text-xs font-bold text-slate-500">
            {t(locale, "customer.noResults")}
          </p>
        )}

        <div className="flex max-h-56 flex-col gap-1 overflow-y-auto">
          {visible.map((customer) => (
            <div
              key={customer.id}
              className="flex items-stretch gap-1 rounded-md bg-slate-100 dark:bg-slate-800"
            >
              <button
                type="button"
                disabled={selecting}
                onClick={() => {
                  selectCustomer(customer.id);
                }}
                className="flex min-w-0 flex-1 items-center justify-between gap-2 px-2 py-2 text-start hover:bg-slate-200 disabled:opacity-50 dark:hover:bg-slate-700"
              >
                <span className="min-w-0">
                  <span className="flex flex-wrap items-center gap-1">
                    <span className="block truncate text-sm font-extrabold">{customer.name}</span>
                    <CustomerTierBadge tier={customer.customer_tier} locale={locale} />
                  </span>
                  {customer.phone !== null && (
                    <span className="block font-mono text-[11px] opacity-60" dir="ltr">
                      {customer.phone}
                    </span>
                  )}
                </span>
                <span className="shrink-0 rounded-full bg-slate-300 px-2 py-0.5 font-mono text-[10px] font-black dark:bg-slate-600">
                  {customer.credit_policy}
                </span>
              </button>
              {customer.credit_policy !== "DISABLED" && (
                <button
                  type="button"
                  title={t(locale, "payment.openButton")}
                  aria-label={t(locale, "payment.openButton")}
                  disabled={selecting || payingId !== null}
                  onClick={() => {
                    openVersementFor(customer.id);
                  }}
                  className="shrink-0 rounded-md bg-accent-500 px-3 text-base font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
                >
                  {payingId === customer.id ? "…" : "د.ج"}
                </button>
              )}
            </div>
          ))}
        </div>

        {canCreate(role) && (
          <div className="flex flex-col gap-2 rounded-lg border border-dashed border-slate-300 p-2 dark:border-slate-600">
            <button
              type="button"
              onClick={() => {
                setShowCreate((value) => !value);
              }}
              className="h-10 rounded-lg bg-primary-600 text-sm font-black text-white"
            >
              {t(locale, "customer.newTitle")}
            </button>
            {showCreate && (
              <>
                <input
                  value={name}
                  onChange={(event) => {
                    setName(event.target.value);
                  }}
                  placeholder={t(locale, "customer.nameLabel")}
                  autoComplete="off"
                  className={inputClass}
                />
                <input
                  value={phone}
                  onChange={(event) => {
                    setPhone(event.target.value);
                  }}
                  placeholder="05XXXXXXXX"
                  autoComplete="off"
                  dir="ltr"
                  className={`${inputClass} font-mono`}
                />
                <div className="grid grid-cols-3 gap-2">
                  {(["DISABLED", "LIMITED", "UNLIMITED"] as const).map((option) => (
                    <button
                      key={option}
                      type="button"
                      onClick={() => {
                        setPolicy(option);
                      }}
                      className={`rounded-lg px-2 py-2 text-xs font-black ${
                        policy === option
                          ? "bg-primary-600 text-white"
                          : "bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300"
                      }`}
                    >
                      {option}
                    </button>
                  ))}
                </div>
                <div className="grid grid-cols-3 gap-2">
                  {tierOptions.map((option) => (
                    <button
                      key={option.id}
                      type="button"
                      onClick={() => {
                        setNewTier(option.id);
                      }}
                      className={`rounded-lg px-2 py-2 text-xs font-black ${
                        newTier === option.id
                          ? "bg-amber-500 text-slate-950"
                          : "bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300"
                      }`}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
                {policy === "LIMITED" && canManageCredit(role) && (
                  <input
                    value={limitText}
                    onChange={(event) => {
                      setLimitText(event.target.value);
                    }}
                    placeholder={t(locale, "customer.limitLabel")}
                    inputMode="decimal"
                    autoComplete="off"
                    className={`${inputClass} font-mono`}
                  />
                )}
                {formError !== null && (
                  <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                    {formError}
                  </p>
                )}
                <button
                  type="button"
                  onClick={createAndSelect}
                  disabled={creating}
                  className="h-11 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
                >
                  {creating ? t(locale, "customer.creating") : t(locale, "customer.create")}
                </button>
              </>
            )}
          </div>
        )}
      </div>
    </ModalShell>
  );
}
