import {
  PieChart,
  Pie,
  Cell,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';
import { StatsResponse } from '@/types/stats';
import { CHART_SERIES, foldIntoOther } from '@/lib/chartPalette';

interface CategoryPieChartProps {
  data: StatsResponse['contractsByCategory'];
}

const COLORS = CHART_SERIES;

const CategoryPieChart: React.FC<CategoryPieChartProps> = ({ data: rawData }) => {
  const data = foldIntoOther(rawData);
  return (
    <div className="bg-card rounded-lg border border-border p-6 h-full flex flex-col">
      <h3 className="text-lg font-semibold text-foreground mb-4">
        Contracts by category
      </h3>
      <div className="flex-1 min-h-[300px]">
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie
              data={data}
              cx="50%"
              cy="50%"
              labelLine={false}
              outerRadius={80}
              fill={CHART_SERIES[0]}
              dataKey="count"
              nameKey="category"
            >
              {data.map((entry, index) => (
                <Cell
                  key={`cell-${index}`}
                  fill={COLORS[index % COLORS.length]}
                />
              ))}
            </Pie>
            <Tooltip
              contentStyle={{
                backgroundColor: 'var(--card)',
                color: 'var(--card-foreground)',
                borderRadius: '8px',
                border: '1px solid var(--border)',
              }}
            />
            <Legend verticalAlign="bottom" height={36} />
          </PieChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

export default CategoryPieChart;
