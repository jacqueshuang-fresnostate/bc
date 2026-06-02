import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { RefreshCcw } from 'lucide-react';
import { MetricCard } from '../components/MetricCard';
import { ModulePanel } from '../components/ModulePanel';
import type { DashboardSummary, DrawSchedule } from '../types/dashboard';
import { formatMoney } from '../utils/format';

interface DashboardPageProps {
  data: DashboardSummary | null;
  error: string | null;
  loading: boolean;
  onOpenModule: (moduleKey: string) => void;
  onRefresh: () => void;
}

export function DashboardPage({
  data,
  error,
  loading,
  onOpenModule,
  onRefresh,
}: DashboardPageProps) {
  if (loading && !data) {
    return (
      <div className="grid min-h-[420px] place-items-center">
        <Spin size="large" tip="正在加载管理后台概览" />
      </div>
    );
  }

  if (error && !data) {
    return (
      <Banner
        type="danger"
        title="后端接口暂不可用"
        description={`${error}。请确认后端服务已启动，且 VITE_API_BASE_URL 指向正确端口。`}
      />
    );
  }

  if (!data) {
    return null;
  }

  return (
    <div className="space-y-6">
      {error ? (
        <Banner
          type="warning"
          title="刷新失败"
          description={`当前展示上一次成功数据。错误：${error}`}
        />
      ) : null}

      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">系统概览</h1>
          <p className="mt-1 text-sm text-slate-500">
            示例数据来自 `/api/admin/dashboard`，后续接入数据库后替换为真实统计。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={onRefresh}>
          刷新
        </Button>
      </section>

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {data.metrics.map((metric) => (
          <MetricCard
            key={metric.key}
            label={metric.label}
            value={metric.value}
            trend={metric.trend}
          />
        ))}
      </section>

      <section className="grid gap-4 xl:grid-cols-[1.4fr_1fr]">
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold text-ink">彩种与开奖源</h2>
            <Tag color="cyan">{data.lotteries.length} 个彩种</Tag>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full min-w-[680px] text-left text-sm">
              <thead className="border-b border-line text-xs text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">彩种</th>
                  <th className="py-2 pr-4 font-medium">号码类型</th>
                  <th className="py-2 pr-4 font-medium">开奖模式</th>
                  <th className="py-2 pr-4 font-medium">开奖时间</th>
                  <th className="py-2 pr-4 font-medium">合买</th>
                </tr>
              </thead>
              <tbody>
                {data.lotteries.map((lottery) => (
                  <tr key={lottery.id} className="border-b border-slate-100">
                    <td className="py-3 pr-4 font-medium text-ink">{lottery.name}</td>
                    <td className="py-3 pr-4 text-slate-600">
                      {lottery.numberType === 'threeDigit' ? '3 位号码' : '5 位号码'}
                    </td>
                    <td className="py-3 pr-4">
                      <Tag color={drawModeColor(lottery.drawMode)}>
                        {drawModeText(lottery.drawMode)}
                      </Tag>
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {scheduleText(lottery.schedule)}
                    </td>
                    <td className="py-3 pr-4">
                      <Tag color={lottery.groupBuy.enabled ? 'green' : 'grey'}>
                        {lottery.groupBuy.enabled ? '已开启' : '已关闭'}
                      </Tag>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>

        <Card className="rounded-md border border-line">
          <h2 className="mb-3 text-base font-semibold text-ink">运营提示</h2>
          <div className="space-y-3 text-sm text-slate-600">
            <div className="rounded-md bg-slate-50 p-3">
              邮箱注册：
              {data.registration.emailEnabled ? '已开启' : '未开启'}
            </div>
            <div className="rounded-md bg-slate-50 p-3">
              代理邀请：
              {data.invitePolicy.agentsCanInvite ? '仅代理可邀请' : '暂未开启'}
            </div>
            <div className="rounded-md bg-slate-50 p-3">
              默认充值返利：{data.invitePolicy.defaultRechargeRebateBasisPoints / 100}%
            </div>
            <div className="rounded-md bg-slate-50 p-3">
              待提现：{formatMoney(data.finance.pendingWithdrawMinor)}
            </div>
          </div>
        </Card>
      </section>

      <Card className="rounded-md border border-line">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-base font-semibold text-ink">最近订单</h2>
          <Tag color="blue">{data.recentOrders.length} 笔</Tag>
        </div>
        {data.recentOrders.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full min-w-[760px] text-left text-sm">
              <thead className="border-b border-line text-xs text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">订单</th>
                  <th className="py-2 pr-4 font-medium">用户</th>
                  <th className="py-2 pr-4 font-medium">彩种</th>
                  <th className="py-2 pr-4 font-medium">玩法</th>
                  <th className="py-2 pr-4 font-medium">注数</th>
                  <th className="py-2 pr-4 font-medium">金额</th>
                  <th className="py-2 pr-4 font-medium">状态</th>
                </tr>
              </thead>
              <tbody>
                {data.recentOrders.map((order) => (
                  <tr key={order.id} className="border-b border-slate-100">
                    <td className="py-3 pr-4">
                      <div className="font-semibold text-ink">{order.id}</div>
                      <div className="mt-1 text-xs text-slate-400">{order.issue}</div>
                    </td>
                    <td className="py-3 pr-4 text-slate-600">{order.userId}</td>
                    <td className="py-3 pr-4 text-slate-600">{order.lotteryName}</td>
                    <td className="py-3 pr-4 text-slate-600">{order.ruleCode}</td>
                    <td className="py-3 pr-4 text-slate-600">{order.stakeCount} 注</td>
                    <td className="py-3 pr-4 text-slate-600">
                      {formatMoney(order.amountMinor)}
                    </td>
                    <td className="py-3 pr-4">
                      <Tag color={order.status === 'pendingDraw' ? 'blue' : 'grey'}>
                        {order.status === 'pendingDraw' ? '待开奖' : order.status}
                      </Tag>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="rounded-md border border-line p-4 text-sm text-slate-500">
            暂无订单。进入订单管理创建测试投注单后，这里会显示最近订单。
          </div>
        )}
      </Card>

      <section className="space-y-6">
        {data.moduleGroups.map((group) => (
          <ModulePanel
            key={group.key}
            group={group}
            onOpenModule={onOpenModule}
          />
        ))}
      </section>
    </div>
  );
}

function drawModeText(mode: string) {
  const labels: Record<string, string> = {
    platform: '平台开奖',
    api: 'API 接口',
    manual: '指定号码',
  };
  return labels[mode] ?? mode;
}

function drawModeColor(mode: string) {
  const colors: Record<string, 'green' | 'blue' | 'orange'> = {
    platform: 'green',
    api: 'blue',
    manual: 'orange',
  };
  return colors[mode] ?? 'blue';
}

function scheduleText(schedule: DrawSchedule) {
  if ('periodic' in schedule) {
    return `${schedule.periodic.intervalSeconds} 秒一期`;
  }
  if ('daily' in schedule) {
    return `每日 ${schedule.daily.time}`;
  }
  return `${schedule.weekly.weekdays.join('、')} ${schedule.weekly.time}`;
}
