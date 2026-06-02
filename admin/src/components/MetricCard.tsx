import { Card } from '@douyinfe/semi-ui';

interface MetricCardProps {
  label: string;
  value: string;
  trend: string;
}

export function MetricCard({ label, value, trend }: MetricCardProps) {
  return (
    <Card shadows="hover" className="min-h-[116px] rounded-md border border-line">
      <div className="space-y-2">
        <div className="text-sm font-medium text-slate-500">{label}</div>
        <div className="text-2xl font-semibold text-ink">{value}</div>
        <div className="text-xs text-slate-500">{trend}</div>
      </div>
    </Card>
  );
}
