/** Deterministic DZD formatting. Input is integer centimes; never float. */

const dzdFormatter = new Intl.NumberFormat("fr-DZ", {
  style: "currency",
  currency: "DZD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

/** Format integer centimes as a French DZD string, e.g. 10050 -> "100,50 DZD". */
export function formatDzd(centimes: number): string {
  if (!Number.isInteger(centimes)) {
    throw new RangeError("formatDzd expects integer centimes");
  }
  return dzdFormatter.format(centimes / 100);
}
