import { StatsResponse } from '@/types/stats';
import { FileText, CheckCircle2, Users } from 'lucide-react';

interface StatsSummaryCardsProps {
  data: StatsResponse;
}

const StatsSummaryCards: React.FC<StatsSummaryCardsProps> = ({ data }) => {
  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      <div className="rounded-lg border border-border p-6 bg-card">
        <div className="flex items-center justify-between">
          <div>
            <p className="eyebrow">Total contracts</p>
            <h3 className="text-2xl font-semibold font-mono tabular-nums text-foreground mt-1">
              {data.totalContracts.toLocaleString()}
            </h3>
          </div>
          <div className="p-3 bg-primary/10 rounded-md">
            <FileText className="w-6 h-6 text-primary" />
          </div>
        </div>
      </div>

      <div className="rounded-lg border border-border p-6 bg-card">
        <div className="flex items-center justify-between">
          <div>
            <p className="eyebrow">Verified contracts</p>
            <h3 className="text-2xl font-semibold font-mono tabular-nums text-foreground mt-1">
              {data.verifiedPercentage}%
            </h3>
          </div>
          <div className="p-3 bg-success/10 rounded-md">
            <CheckCircle2 className="w-6 h-6 text-success" />
          </div>
        </div>
      </div>

      <div className="rounded-lg border border-border p-6 bg-card">
        <div className="flex items-center justify-between">
          <div>
            <p className="eyebrow">Total publishers</p>
            <h3 className="text-2xl font-semibold font-mono tabular-nums text-foreground mt-1">
              {data.totalPublishers.toLocaleString()}
            </h3>
          </div>
          <div className="p-3 bg-muted rounded-md">
            <Users className="w-6 h-6 text-muted-foreground" />
          </div>
        </div>
      </div>
    </div>
  );
};

export default StatsSummaryCards;
