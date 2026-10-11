/**
 * Pure document helpers (TASK-DSN-028): canonical type/status codes with
 * badge classes and i18n-key resolution. Money stays in canonical
 * centime strings — zero floats anywhere in this module.
 */

/** Selling-side document kinds exposed by the filter pills. */
export const DOCUMENT_TYPES = ["TICKET", "BL", "FACTURE", "DEVIS", "AVOIR"] as const;
export type DocumentTypeFilter = (typeof DOCUMENT_TYPES)[number] | "ALL";

/** Lifecycle states exposed by the status selector. */
export const DOCUMENT_STATUSES = ["FINALIZED", "CONVERTED", "DRAFT", "CANCELLED"] as const;
export type DocumentStatusFilter = (typeof DOCUMENT_STATUSES)[number] | "ALL";

/** i18n key for the localized type pill/badge label. */
export function documentTypeKey(
  documentType: string,
): "documents.typeTicket" | "documents.typeBl" | "documents.typeFacture" | "documents.typeDevis" | "documents.typeAvoir" | "documents.typeAll" {
  switch (documentType) {
    case "TICKET":
      return "documents.typeTicket";
    case "BL":
      return "documents.typeBl";
    case "FACTURE":
      return "documents.typeFacture";
    case "DEVIS":
      return "documents.typeDevis";
    case "AVOIR":
      return "documents.typeAvoir";
    default:
      return "documents.typeAll";
  }
}

/** Semantic badge classes per document type (light + dark). */
export function documentTypeBadge(documentType: string): string {
  switch (documentType) {
    case "TICKET":
      return "bg-slate-200 text-slate-800 dark:bg-slate-700 dark:text-slate-100";
    case "BL":
      return "bg-sky-100 text-sky-800 dark:bg-sky-900/50 dark:text-sky-200";
    case "FACTURE":
      return "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/50 dark:text-indigo-200";
    case "DEVIS":
      return "bg-amber-100 text-amber-900 dark:bg-amber-900/50 dark:text-amber-200";
    case "AVOIR":
      return "bg-rose-100 text-rose-800 dark:bg-rose-900/50 dark:text-rose-200";
    default:
      return "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200";
  }
}

/** i18n key for the localized status badge label. */
export function documentStatusKey(
  status: string,
): "documents.statusFinalized" | "documents.statusConverted" | "documents.statusDraft" | "documents.statusCancelled" | "documents.statusAll" {
  switch (status) {
    case "FINALIZED":
      return "documents.statusFinalized";
    case "CONVERTED":
      return "documents.statusConverted";
    case "DRAFT":
      return "documents.statusDraft";
    case "CANCELLED":
      return "documents.statusCancelled";
    default:
      return "documents.statusAll";
  }
}

/** Semantic badge classes per lifecycle status (light + dark). */
export function documentStatusBadge(status: string): string {
  switch (status) {
    case "FINALIZED":
      return "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/50 dark:text-emerald-200";
    case "CONVERTED":
      return "bg-sky-100 text-sky-800 dark:bg-sky-900/50 dark:text-sky-200";
    case "DRAFT":
      return "bg-amber-100 text-amber-900 dark:bg-amber-900/50 dark:text-amber-200";
    case "CANCELLED":
      return "bg-slate-300 text-slate-700 dark:bg-slate-700 dark:text-slate-300";
    default:
      return "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200";
  }
}
