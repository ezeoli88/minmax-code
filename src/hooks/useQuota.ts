import { useState, useEffect, useCallback } from "react";
import { fetchCodingPlanRemains, type QuotaInfo } from "../core/api.js";

export function useQuota(apiKey: string) {
  const [quota, setQuota] = useState<QuotaInfo | null>(null);

  const refresh = useCallback(async () => {
    if (!apiKey) return;
    try {
      const info = await fetchCodingPlanRemains(apiKey);
      if (info) setQuota(info);
    } catch {
      // silently ignore fetch errors
    }
  }, [apiKey]);

  // Fetch on mount + slow fallback poll (5 min) in case event-driven refresh misses
  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 300_000);
    return () => clearInterval(id);
  }, [refresh]);

  return { quota, refreshQuota: refresh };
}
