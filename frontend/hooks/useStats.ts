import { useState, useEffect, useCallback } from 'react';
import { fetchStats } from '@/lib/api/stats';
import { StatsResponse, TimePeriod } from '@/types/stats';

interface UseStatsReturn {
  data: StatsResponse | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

export function useStats(period: TimePeriod): UseStatsReturn {
  const [data, setData] = useState<StatsResponse | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<Error | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await fetchStats(period);
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch stats'));
    } finally {
      setLoading(false);
    }
  }, [period]);

  useEffect(() => {
    loadData();

    const intervalId = setInterval(() => {
      // Background refresh without setting loading state
      fetchStats(period)
        .then(setData)
        .catch((err) => console.error('Polling error:', err));
    }, 30000);

    return () => clearInterval(intervalId);
  }, [loadData, period]);

  return { data, loading, error, refetch: loadData };
}
