/**
 * Pure users-management helpers (BRK-064 engine contract ↔ DSN-030 UI).
 *
 * Engine reference (mirrors `CreateUserUseCase` validation):
 * - username: 3–30 chars, ASCII letters/digits/underscore (`is_valid_username`)
 * - password: min 6 chars (`is_valid_password`)
 * - PIN: 4–6 ASCII digits (`is_valid_pin`)
 */

import type { Locale } from "@/types/pos";
import type { UserInfo } from "@/lib/ipc";

export type ManagedRole = "ADMIN" | "GERANT" | "CAISSIER" | "VENDEUR";

const ROLE_CODES: readonly ManagedRole[] = ["ADMIN", "GERANT", "CAISSIER", "VENDEUR"];

/** Canonical role codes accepted by `users_create` / `users_update`. */
export function roleOptions(): readonly ManagedRole[] {
  return ROLE_CODES;
}

/** True for a canonical engine role code (wire values, uppercase). */
export function isRoleCode(value: string): value is ManagedRole {
  return (ROLE_CODES as readonly string[]).includes(value);
}

/** Display label for a role code (spec '4 canonical options' surface). */
export function roleLabel(role: string, locale: Locale): string {
  switch (role) {
    case "ADMIN":
      return locale === "ar-DZ" ? "مدير عام" : "Administrateur";
    case "GERANT":
      return locale === "ar-DZ" ? "مسيّر" : "Gérant";
    case "CAISSIER":
      return locale === "ar-DZ" ? "أمين صندوق" : "Caissier";
    case "VENDEUR":
      return locale === "ar-DZ" ? "بائع" : "Vendeur";
    default:
      return role;
  }
}

/**
 * Tailwind badge classes for a role (DSN-071: semantic status tokens,
 * WCAG AA in both themes — ADMIN neutral-strong, others status-mirrored).
 */
export function roleBadgeClass(role: string): string {
  switch (role) {
    case "ADMIN":
      return "border border-line-strong bg-surface-sunken font-semibold text-ink-primary";
    case "GERANT":
      return "border border-info-fg/20 bg-info-bg font-semibold text-info-fg";
    case "CAISSIER":
      return "border border-warning-fg/20 bg-warning-bg font-semibold text-warning-fg";
    case "VENDEUR":
      return "border border-success-fg/20 bg-success-bg font-semibold text-success-fg";
    default:
      return "border border-line-default bg-surface-sunken text-ink-muted";
  }
}

/**
 * Tailwind badge classes for the active/inactive status pill. Inactive uses
 * secondary (not muted) text: muted-on-sunken measures 4.23:1 in light mode
 * (AA fail); secondary-on-sunken measures 9.20:1.
 */
export function statusBadgeClass(isActive: boolean): string {
  return isActive
    ? "bg-success-bg font-semibold text-success-fg"
    : "border border-line-default bg-surface-sunken text-ink-secondary";
}

/** Username rule (mirrors `is_valid_username`): 3–30 ASCII letters/digits/underscore. */
export function validateUsername(value: string): boolean {
  if (value.length < 3 || value.length > 30) {
    return false;
  }
  return /^[A-Za-z0-9_]+$/.test(value);
}

/** Password rule (mirrors `is_valid_password`): min 6 chars. */
export function validatePassword(value: string): boolean {
  return value.length >= 6;
}

/** PIN rule (mirrors `is_valid_pin`): 4–6 ASCII digits. */
export function validatePin(value: string): boolean {
  return /^[0-9]{4,6}$/.test(value);
}

export interface UserMetrics {
  total: number;
  admins: number;
  cashAndSales: number;
  inactive: number;
}

/** HUD counts: total, admins, CAISSIER+VENDEUR, inactive. */
export function computeUserMetrics(users: readonly UserInfo[]): UserMetrics {
  return {
    total: users.length,
    admins: users.filter((user) => user.role === "ADMIN").length,
    cashAndSales: users.filter((user) => user.role === "CAISSIER" || user.role === "VENDEUR")
      .length,
    inactive: users.filter((user) => !user.is_active).length,
  };
}

/**
 * Localized, human-friendly message for a backend conflict code. Prefers the
 * 4 contract codes (`LAST_ADMIN_PROTECTED`, `USERNAME_TAKEN`, `INVALID_PIN`,
 * `WEAK_PASSWORD`); falls back to the raw code so nothing is swallowed.
 */
export function userErrorMessage(code: string, locale: Locale): string {
  switch (code) {
    case "LAST_ADMIN_PROTECTED":
      return locale === "ar-DZ"
        ? "لا يمكن تعطيل أو خفض صلاحيات آخر مدير عام نشط"
        : "Impossible de désactiver ou rétrograder le dernier administrateur actif";
    case "USERNAME_TAKEN":
      return locale === "ar-DZ" ? "اسم المستخدم مستعمل مسبقاً" : "Nom d'utilisateur déjà utilisé";
    case "INVALID_PIN":
      return locale === "ar-DZ"
        ? "رمز PIN غير صالح (4 إلى 6 أرقام)"
        : "PIN invalide (4 à 6 chiffres)";
    case "WEAK_PASSWORD":
      return locale === "ar-DZ"
        ? "كلمة المرور ضعيفة (6 أحرف على الأقل)"
        : "Mot de passe trop faible (6 caractères min)";
    case "INVALID_USERNAME":
      return locale === "ar-DZ"
        ? "اسم المستخدم غير صالح (3 إلى 30 حرفاً: أحرف وأرقام و_)"
        : "Nom d'utilisateur invalide (3 à 30 caractères : lettres, chiffres, _)";
    case "NOTHING_TO_RESET":
      return locale === "ar-DZ"
        ? "قدّم كلمة مرور أو رمز PIN جديداً واحداً على الأقل"
        : "Fournissez au moins un nouveau mot de passe ou PIN";
    default:
      return code;
  }
}
