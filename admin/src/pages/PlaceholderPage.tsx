import { Banner, Card, Tag } from '@douyinfe/semi-ui';
import type { DashboardSummary, ModuleGroup } from '../types/dashboard';

interface PlaceholderPageProps {
  moduleKey: string;
  summary: DashboardSummary;
}

export function PlaceholderPage({ moduleKey, summary }: PlaceholderPageProps) {
  const moduleInfo = findModule(summary.moduleGroups, moduleKey);

  if (!moduleInfo) {
    return (
      <Banner
        type="warning"
        title="未找到模块"
        description="该入口尚未出现在后端概览数据中。"
      />
    );
  }

  return (
    <div className="space-y-4">
      <div>
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-semibold text-ink">{moduleInfo.name}</h1>
          <Tag color={moduleInfo.status === 'scaffolded' ? 'green' : 'orange'}>
            {moduleInfo.status === 'scaffolded' ? '已建入口' : '待实现'}
          </Tag>
        </div>
        <p className="mt-1 text-sm text-slate-500">{moduleInfo.description}</p>
      </div>
      <Card className="rounded-md border border-line">
        <div className="space-y-2">
          <div className="text-sm font-semibold text-ink">当前状态</div>
          <p className="text-sm leading-6 text-slate-600">
            此页面已纳入管理后台导航，后续会在对应业务任务中补充列表、筛选、表单、权限和真实接口。
          </p>
        </div>
      </Card>
    </div>
  );
}

function findModule(groups: ModuleGroup[], moduleKey: string) {
  for (const group of groups) {
    const moduleInfo = group.modules.find((item) => item.key === moduleKey);
    if (moduleInfo) {
      return moduleInfo;
    }
  }
  return null;
}
