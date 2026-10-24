/**
 * Inline SVG icon set for the application shell (TASK-DSN-090).
 * Owner direction: icon+text on every button, one visual language.
 * 1.75px stroke, currentColor, no emoji. Directional icons opt into
 * RTL mirroring via `flipRtl` (Tailwind `rtl:-scale-x-100`).
 */

import type { ReactElement } from "react";

export interface IconProps {
  className?: string;
  /** Mirror horizontally under `[dir="rtl"]` (directional icons only). */
  flipRtl?: boolean;
}

function Base({
  className,
  flipRtl = false,
  children,
}: IconProps & { children: React.ReactNode }): ReactElement {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={`${className ?? "h-4 w-4"}${flipRtl ? " rtl:-scale-x-100" : ""}`}
    >
      {children}
    </svg>
  );
}

/** Cash session (banknote). Used: Topbar cash pill. */
export function CashIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <rect x="2" y="6" width="20" height="12" rx="2" />
      <circle cx="12" cy="12" r="2.5" />
      <path d="M6 12h.01M18 12h.01" />
    </Base>
  );
}

/** Depot/warehouse. Used: Topbar depot selector label. */
export function DepotIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M3 9.5 12 3l9 6.5" />
      <path d="M5 10v10h14V10" />
      <path d="M9 20v-6h6v6" />
    </Base>
  );
}

/** Operations menu (sliders). Used: Topbar OperationsMenu trigger. */
export function OperationsIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0" />
      <circle cx="16" cy="6" r="2" />
      <circle cx="10" cy="12" r="2" />
      <circle cx="18" cy="18" r="2" />
    </Base>
  );
}

/** Operator / user. Used: Topbar session badge, nav Users entry. */
export function UserIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="12" cy="8" r="3.5" />
      <path d="M4.5 20c1.5-3.5 4.5-5 7.5-5s6 1.5 7.5 5" />
    </Base>
  );
}

/** Live clock. Used: Topbar clock chip. */
export function ClockIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 2" />
    </Base>
  );
}

/** Language (globe). Used: Topbar locale pill. */
export function LangIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17M12 3.5c2.5 2.5 3.8 5.4 3.8 8.5s-1.3 6-3.8 8.5c-2.5-2.5-3.8-5.4-3.8-8.5s1.3-6 3.8-8.5Z" />
    </Base>
  );
}

/** Moon (dark mode target). Used: Topbar theme toggle. */
export function MoonIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5Z" />
    </Base>
  );
}

/** Sun (light mode target). Used: Topbar theme toggle. */
export function SunIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5 5l1.8 1.8M17.2 17.2 19 19M19 5l-1.8 1.8M6.8 17.2 5 19" />
    </Base>
  );
}

/** Backup (shield). Used: OperationsMenu backup entry. */
export function ShieldIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M12 2.5 20 6v6c0 5-3.5 8.5-8 9.5C7.5 20.5 4 17 4 12V6l8-3.5Z" />
      <path d="M8.8 12l2.2 2.2 4.2-4.2" />
    </Base>
  );
}

/** Printer. Used: OperationsMenu identity/test-print entry. */
export function PrinterIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M6 9V2.5h12V9" />
      <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
      <rect x="6" y="14" width="12" height="7.5" />
    </Base>
  );
}
/** Chevron down (directional — identical in RTL). Used: cash + operations menu toggles. */
export function ChevronDownIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M6 9l6 6 6-6" />
    </Base>
  );
}

/** Logout (directional — flips in RTL). Used: Topbar logout. */
export function LogoutIcon(props: IconProps): ReactElement {
  return (
    <Base flipRtl {...props}>
      <path d="M14 4h5a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-5" />
      <path d="M10 8l4 4-4 4M14 12H3" />
    </Base>
  );
}

/** POS register (cart). Used: nav Caisse entry. */
export function PosIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M3 4h2l2.4 12.2a1 1 0 0 0 1 .8h9.9a1 1 0 0 0 1-.8L21.5 8H6" />
      <circle cx="9.5" cy="20" r="1.3" />
      <circle cx="17.5" cy="20" r="1.3" />
    </Base>
  );
}

/** Catalog (tag). Used: nav Catalogue entry. */
export function CatalogIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M3.5 3.5h7.2L21 13.8a1.5 1.5 0 0 1 0 2.1l-5.1 5.1a1.5 1.5 0 0 1-2.1 0L3.5 10.7V3.5Z" />
      <circle cx="8" cy="8" r="1.3" />
    </Base>
  );
}

/** Stock (package). Used: nav Stock entry. */
export function StockIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M12 2.5 21 7.5v9l-9 5-9-5v-9l9-5Z" />
      <path d="M3.5 7.5 12 12l8.5-4.5M12 12v9.5" />
    </Base>
  );
}

/** Customers (two users). Used: nav Clients entry. */
export function CustomersIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="9" cy="8" r="3" />
      <path d="M2.5 19.5c1.2-3 3.6-4.5 6.5-4.5s5.3 1.5 6.5 4.5" />
      <path d="M15.5 5.5a3 3 0 0 1 0 5.8M17.5 15.3c2 .7 3.4 2 4 4.2" />
    </Base>
  );
}

/** Documents (file text). Used: nav Documents entry. */
export function DocumentsIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M6 2.5h8l4 4v15H6V2.5Z" />
      <path d="M14 2.5v4h4M9 12h6M9 15.5h6" />
    </Base>
  );
}

/** Suppliers (truck, directional — flips in RTL). Used: nav Fournisseurs. */
export function SuppliersIcon(props: IconProps): ReactElement {
  return (
    <Base flipRtl {...props}>
      <path d="M1.5 5.5h13v11h-13zM14.5 9.5h4l3 3.5v3.5h-7" />
      <circle cx="6" cy="18.5" r="1.8" />
      <circle cx="17.5" cy="18.5" r="1.8" />
    </Base>
  );
}

/** Reports (bar chart). Used: nav Rapports entry. */
export function ReportsIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <path d="M3.5 3.5v17h17" />
      <path d="M8 15.5v-5M12.5 15.5v-8M17 15.5V7" />
    </Base>
  );
}

/** Settings (gear: tangent teeth + hub — never sun rays). Used: nav Paramètres entry. */
export function SettingsIcon(props: IconProps): ReactElement {
  return (
    <Base {...props}>
      <circle cx="12" cy="12" r="3" />
      <path d="M18 10.8v2.4M15.4 17.1l1.7-1.7M10.8 18h2.4M6.9 15.4l1.7 1.7M6 10.8v2.4M8.6 6.9l-1.7 1.7M10.8 6h2.4M17.1 8.6l-1.7-1.7" />
    </Base>
  );
}
