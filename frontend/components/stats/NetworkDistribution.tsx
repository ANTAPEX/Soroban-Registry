import React from 'react';
import { StatsResponse } from '@/types/stats';

interface NetworkDistributionProps {
  data: StatsResponse['networkBreakdown'];
}

const NetworkDistribution: React.FC<NetworkDistributionProps> = ({ data }) => {
  const total = data.reduce((acc, curr) => acc + curr.contracts, 0);

  return (
    <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6 h-full">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
        Network Distribution
      </h3>
      <div className="space-y-4">
        {data.map((item) => {
          const percentage = total > 0 ? (item.contracts / total) * 100 : 0;
          return (
            <div key={item.network}>
              <div className="flex justify-between items-center mb-1">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  {item.network}
                </span>
                <span className="text-sm text-gray-500 dark:text-gray-400">
                  {item.contracts} ({percentage.toFixed(1)}%)
                </span>
              </div>
              <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5">
                <div
                  className="bg-blue-600 h-2.5 rounded-full"
                  style={{ width: `${percentage}%` }}
                ></div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default NetworkDistribution;
