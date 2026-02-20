import { useState, useEffect, useCallback } from "react";
import { fetchCodingPlanRemains, type QuotaInfo } from "../core/api.js";

const POLL_INTERVAL = 60_000; // refresh every 60s

export function useQuota(apiKey: string) {
  const [quota, setQuota] = useState<QuotaInfo | null>(null);

  const refresh = useCallback(async () => {
    if (!apiKey) return;
    const info = await fetchCodingPlanRemains(apiKey);
    if (info) setQuota(info);
  }, [apiKey]);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, POLL_INTERVAL);
    return () => clearInterval(id);
  }, [refresh]);

  return { quota, refreshQuota: refresh };
}
