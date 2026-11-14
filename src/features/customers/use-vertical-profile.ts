/**
 * Active vertical profile shared hook (DSN-105): one fetch per mount,
 * silent collapse on failure (callers fall back to the full tier list).
 * Money never crosses this edge — codes only.
 */
import { useEffect, useState } from "react";
import { getVerticalProfile } from "@/lib/ipc";
import type { VerticalProfileInfo } from "@/types/pos";

const FALLBACK_PROFILE: VerticalProfileInfo = {
  profile: "apparel_wholesale_retail",
  offeredTiers: ["retail", "artisan", "wholesale", "super_gros"],
  defaultTier: "retail",
};

export function useVerticalProfile(): {
  profile: VerticalProfileInfo;
  loading: boolean;
  reload: () => void;
} {
  const [profile, setProfile] = useState<VerticalProfileInfo>(FALLBACK_PROFILE);
  const [loading, setLoading] = useState(true);
  const [reloadTick, setReloadTick] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    getVerticalProfile()
      .then((snapshot) => {
        if (!cancelled) {
          setProfile(snapshot);
          setLoading(false);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setProfile(FALLBACK_PROFILE);
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [reloadTick]);

  return { profile, loading, reload: () => setReloadTick((tick) => tick + 1) };
}

/** Canonical profile code → settings label key. Unknown codes read as apparel. */
export function profileLabelKey(code: string): string {
  switch (code) {
    case "hardware_general":
      return "settings.profileHardware";
    case "hybrid_enterprise":
      return "settings.profileHybrid";
    default:
      return "settings.profileApparel";
  }
}

/** Tier code → `tier_*` label key (unknown codes read as retail). */
export function tierLabelKey(
  tier: string,
): "tier_retail" | "tier_artisan" | "tier_wholesale" | "tier_super_gros" {
  switch (tier) {
    case "artisan":
      return "tier_artisan";
    case "wholesale":
      return "tier_wholesale";
    case "super_gros":
      return "tier_super_gros";
    default:
      return "tier_retail";
  }
}
