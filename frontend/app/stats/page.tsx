'use client';

import React, { useState } from 'react';
import { useStats } from '@/hooks/useStats';
import { TimePeriod } from '@/types/stats';
import StatsSummaryCards from '@/components/stats/StatsSummaryCards';
import CategoryPieChart from '@/components/stats/CategoryPieChart';
import DeploymentsTrendChart from '@/components/stats/DeploymentsTrendChart';
import TopPublishersTable from '@/components/stats/TopPublishersTable';
import NetworkDistribution from '@/components/stats/NetworkDistribution';
import TimePeriodSelector from '@/components/stats/TimePeriodSelector';
import StatsSkeleton from '@/components/stats/StatsSkeleton';
import { AlertCircle, RefreshCw } from 'lucide-react';

export default function StatsPage() {
  const [period, setPeriod] = useState<TimePeriod>('30d');
  const { data, loading, error, refetch } = useStats(period);

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900 p-4">
        <div className="bg-white dark:bg-gray-800 p-8 rounded-xl shadow-lg max-w-md w-full text-center border border-red-200 dark:border-red-900">
          <AlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
          <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-2">
            Failed to load statistics
          </h2>
          <p className="text-gray-600 dark:text-gray-400 mb-6">
            {error.message || 'An unexpected error occurred while fetching data.'}
          </p>
          <button
            onClick={() => refetch()}
            className="inline-flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
          >
            <RefreshCw className="w-4 h-4 mr-2" />
            Try Again
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 py-8 px-4 sm:px-6 lg:px-8">
      <div className="max-w-7xl mx-auto space-y-8">
        {/* Header Section */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
              Registry Statistics
            </h1>
            <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
              Overview of Soroban contract deployments and network activity
            </p>
          </div>
          <TimePeriodSelector
            selectedPeriod={period}
            onPeriodChange={setPeriod}
          />
        </div>

        {loading || !data ? (
          <StatsSkeleton />
        ) : (
          <div className="space-y-6 animate-in fade-in duration-500">
            {/* Summary Cards */}
            <StatsSummaryCards data={data} />

            {/* Charts Row */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 h-[400px]">
              <DeploymentsTrendChart data={data.deploymentsTrend} />
              <CategoryPieChart data={data.contractsByCategory} />
            </div>

            {/* Bottom Section */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2">
                <TopPublishersTable data={data.topPublishers} />
              </div>
              <div>
                <NetworkDistribution data={data.networkBreakdown} />
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
