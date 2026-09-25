import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { StatsResponse } from '@/types/stats';
import { CHART_AXIS, CHART_GRID, CHART_SERIES } from '@/lib/chartPalette';

interface DeploymentsTrendChartProps {
  data: StatsResponse['deploymentsTrend'];
}

const DeploymentsTrendChart: React.FC<DeploymentsTrendChartProps> = ({
  data,
}) => {
  return (
    <div className="bg-card rounded-lg border border-border p-6 h-full flex flex-col">
      <h3 className="text-lg font-semibold text-foreground mb-4">
        Deployments trend
      </h3>
      <div className="flex-1 min-h-[300px]">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart
            data={data}
            margin={{
              top: 5,
              right: 30,
              left: 20,
              bottom: 5,
            }}
          >
            <CartesianGrid strokeDasharray="3 3" stroke={CHART_GRID} />
            <XAxis
              dataKey="date"
              tickFormatter={(date) => {
                const d = new Date(date);
                return `${d.getMonth() + 1}/${d.getDate()}`;
              }}
              stroke={CHART_AXIS}
              tick={{ fontSize: 12 }}
            />
            <YAxis stroke={CHART_AXIS} tick={{ fontSize: 12 }} />
            <Tooltip
              contentStyle={{
                backgroundColor: 'var(--card)',
                color: 'var(--card-foreground)',
                borderRadius: '8px',
                border: '1px solid var(--border)',
                boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',
              }}
            />
            <Line
              type="monotone"
              dataKey="count"
              stroke={CHART_SERIES[0]}
              strokeWidth={2}
              activeDot={{ r: 8 }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

export default DeploymentsTrendChart;
