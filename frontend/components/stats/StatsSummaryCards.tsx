import React from 'react';
import { StatsResponse } from '@/types/stats';
import { FileText, CheckCircle2, Users } from 'lucide-react';

interface StatsSummaryCardsProps {
  data: StatsResponse;
}

const StatsSummaryCards: React.FC<StatsSummaryCardsProps> = ({ data }) => {
  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      <div className="rounded-xl border border-gray-200 dark:border-gray-800 p-6 bg-white dark:bg-gray-900 transition-all hover:shadow-lg hover:border-blue-500/50">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-500 dark:text-gray-400">Total Contracts</p>
            <h3 className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {data.totalContracts.toLocaleString()}
            </h3>
          </div>
          <div className="p-3 bg-blue-500/10 rounded-lg">
            <FileText className="w-6 h-6 text-blue-600" />
          </div>
        </div>
      </div>

      <div className="rounded-xl border border-gray-200 dark:border-gray-800 p-6 bg-white dark:bg-gray-900 transition-all hover:shadow-lg hover:border-green-500/50">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-500 dark:text-gray-400">Verified Contracts</p>
            <h3 className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {data.verifiedPercentage}%
            </h3>
          </div>
          <div className="p-3 bg-green-500/10 rounded-lg">
            <CheckCircle2 className="w-6 h-6 text-green-600" />
          </div>
        </div>
      </div>

      <div className="rounded-xl border border-gray-200 dark:border-gray-800 p-6 bg-white dark:bg-gray-900 transition-all hover:shadow-lg hover:border-purple-500/50">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-500 dark:text-gray-400">Total Publishers</p>
            <h3 className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {data.totalPublishers.toLocaleString()}
            </h3>
          </div>
          <div className="p-3 bg-purple-500/10 rounded-lg">
            <Users className="w-6 h-6 text-purple-600" />
          </div>
        </div>
      </div>
    </div>
  );
};

export default StatsSummaryCards;
