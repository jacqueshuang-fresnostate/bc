import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Calculator, RefreshCcw } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useDraws } from '../hooks/useDraws';
import { useSettlements } from '../hooks/useSettlements';
import type { DrawIssue } from '../types/draws';
import type { SettlementRun } from '../types/settlements';
import { formatMoney } from '../utils/format';
import { formatOdds } from '../utils/playRules';

interface SettlementManagementPageProps {
  onDashboardRefresh: () => void;
}

export function SettlementManagementPage({
  onDashboardRefresh,
}: SettlementManagementPageProps) {
  const {
    error: drawError,
    issues,
    loading: drawsLoading,
    refresh: refreshDraws,
  } = useDraws();
  const {
    error: settlementError,
    loading: settlementsLoading,
    refresh: refreshSettlements,
    saving,
    settle,
    settlements,
  } = useSettlements();
  const [selectedDrawIssueId, setSelectedDrawIssueId] = useState<string | null>(null);
  const [selectedSettlementId, setSelectedSettlementId] = useState<string | null>(null);

  const drawnIssues = useMemo(
    () => issues.filter((issue) => issue.status === 'drawn'),
    [issues],
  );
  const selectedDrawIssue = useMemo(
    () =>
      drawnIssues.find((issue) => issue.id === selectedDrawIssueId) ??
      drawnIssues[0] ??
      null,
    [drawnIssues, selectedDrawIssueId],
  );
  const selectedSettlement = useMemo(
    () =>
      settlements.find((settlement) => settlement.id === selectedSettlementId) ??
      settlements[0] ??
      null,
    [settlements, selectedSettlementId],
  );
  const settledDrawIssueIds = useMemo(
    () => new Set(settlements.map((settlement) => settlement.drawIssueId)),
    [settlements],
  );

  useEffect(() => {
    if (selectedDrawIssue && selectedDrawIssue.id !== selectedDrawIssueId) {
      setSelectedDrawIssueId(selectedDrawIssue.id);
    }
  }, [selectedDrawIssue, selectedDrawIssueId]);

  useEffect(() => {
    if (selectedSettlement && selectedSettlement.id !== selectedSettlementId) {
      setSelectedSettlementId(selectedSettlement.id);
    }
  }, [selectedSettlement, selectedSettlementId]);

  const refreshAll = () => {
    refreshDraws();
    refreshSettlements();
  };

  const settleSelectedIssue = async () => {
    if (!selectedDrawIssue) {
      return;
    }
    const settlement = await settle(selectedDrawIssue.id);
    setSelectedSettlementId(settlement.id);
    refreshDraws();
    onDashboardRefresh();
  };

  const loading = drawsLoading || settlementsLoading;
  const error = drawError ?? settlementError;

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">计奖派奖</h1>
          <p className="mt-1 text-sm text-slate-500">
            对已开奖期号执行基础计奖，生成结算批次并更新订单状态。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="计奖派奖接口错误" description={error} /> : null}

      <section className="grid gap-4 xl:grid-cols-[420px_1fr]">
        <Card className="rounded-md border border-line">
          <div className="mb-4 flex items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold text-ink">已开奖期号</h2>
              <p className="mt-1 text-sm text-slate-500">
                选择一个已开奖期号执行基础派奖。
              </p>
            </div>
            <Tag color="cyan">{drawnIssues.length} 个</Tag>
          </div>

          {loading ? (
            <div className="grid min-h-[260px] place-items-center">
              <Spin tip="正在加载结算数据" />
            </div>
          ) : drawnIssues.length > 0 ? (
            <div className="space-y-3">
              <select
                className="form-input"
                value={selectedDrawIssue?.id ?? ''}
                onChange={(event) => setSelectedDrawIssueId(event.target.value)}
              >
                {drawnIssues.map((issue) => (
                  <option key={issue.id} value={issue.id}>
                    {issue.lotteryName} {issue.issue}（{issue.drawNumber ?? '未记录'}）
                  </option>
                ))}
              </select>

              {selectedDrawIssue ? (
                <DrawIssueSummary
                  issue={selectedDrawIssue}
                  settled={settledDrawIssueIds.has(selectedDrawIssue.id)}
                />
              ) : null}

              <Button
                disabled={
                  saving ||
                  !selectedDrawIssue ||
                  settledDrawIssueIds.has(selectedDrawIssue.id)
                }
                icon={<Calculator size={16} />}
                theme="solid"
                onClick={() => void settleSelectedIssue()}
              >
                {saving ? '计奖中' : '执行计奖派奖'}
              </Button>
            </div>
          ) : (
            <div className="rounded-md border border-line p-4 text-sm text-slate-500">
              暂无已开奖期号，请先在“开奖期号与开奖源”页面完成开奖。
            </div>
          )}
        </Card>

        <Card className="rounded-md border border-line">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold text-ink">结算批次</h2>
            <Tag color="cyan">{settlements.length} 个批次</Tag>
          </div>

          {loading ? (
            <div className="grid min-h-[320px] place-items-center">
              <Spin tip="正在加载结算批次" />
            </div>
          ) : settlements.length > 0 ? (
            <div className="space-y-4">
              <div className="overflow-x-auto">
                <table className="w-full min-w-[860px] text-left text-sm">
                  <thead className="border-b border-line text-xs text-slate-500">
                    <tr>
                      <th className="py-2 pr-4 font-medium">批次</th>
                      <th className="py-2 pr-4 font-medium">彩种/期号</th>
                      <th className="py-2 pr-4 font-medium">开奖号码</th>
                      <th className="py-2 pr-4 font-medium">订单</th>
                      <th className="py-2 pr-4 font-medium">派奖</th>
                      <th className="py-2 pr-4 font-medium">操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {settlements.map((settlement) => (
                      <tr
                        key={settlement.id}
                        className={`border-b border-slate-100 ${
                          selectedSettlement?.id === settlement.id ? 'bg-teal-50/60' : ''
                        }`}
                      >
                        <td className="py-3 pr-4">
                          <div className="font-semibold text-ink">{settlement.id}</div>
                          <div className="mt-1 text-xs text-slate-400">
                            {settlement.createdAt}
                          </div>
                        </td>
                        <td className="py-3 pr-4">
                          <div className="font-medium text-ink">{settlement.lotteryName}</div>
                          <div className="mt-1 text-xs text-slate-400">
                            {settlement.issue}
                          </div>
                        </td>
                        <td className="py-3 pr-4 font-mono font-semibold text-ink">
                          {settlement.drawNumber}
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {settlement.settledOrderCount} 单 / {settlement.winningOrderCount} 中
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {formatMoney(settlement.totalPayoutMinor)}
                        </td>
                        <td className="py-3 pr-4">
                          <Button
                            size="small"
                            onClick={() => setSelectedSettlementId(settlement.id)}
                          >
                            查看
                          </Button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {selectedSettlement ? <SettlementDetail settlement={selectedSettlement} /> : null}
            </div>
          ) : (
            <div className="rounded-md border border-line p-4 text-sm text-slate-500">
              暂无结算批次，选择已开奖期号后执行计奖派奖。
            </div>
          )}
        </Card>
      </section>
    </div>
  );
}

function DrawIssueSummary({
  issue,
  settled,
}: {
  issue: DrawIssue;
  settled: boolean;
}) {
  return (
    <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="font-semibold text-ink">{issue.lotteryName}</div>
          <div className="mt-1 text-xs text-slate-400">{issue.id}</div>
        </div>
        <Tag color={settled ? 'green' : 'blue'}>{settled ? '已结算' : '待结算'}</Tag>
      </div>
      <div className="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-1">
        <InfoLine label="期号" value={issue.issue} />
        <InfoLine label="开奖号码" value={issue.drawNumber ?? '-'} />
        <InfoLine label="开奖时间" value={issue.drawnAt ?? issue.scheduledAt} />
      </div>
    </div>
  );
}

function SettlementDetail({ settlement }: { settlement: SettlementRun }) {
  return (
    <div className="rounded-md bg-slate-50 p-3">
      <div className="mb-3 grid gap-3 text-sm sm:grid-cols-3">
        <InfoLine label="投注金额" value={formatMoney(settlement.totalStakeAmountMinor)} />
        <InfoLine label="派奖金额" value={formatMoney(settlement.totalPayoutMinor)} />
        <InfoLine label="中奖订单" value={`${settlement.winningOrderCount} 单`} />
      </div>
      {settlement.orders.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[760px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">订单</th>
                <th className="py-2 pr-4 font-medium">用户</th>
                <th className="py-2 pr-4 font-medium">玩法</th>
                <th className="py-2 pr-4 font-medium">命中</th>
                <th className="py-2 pr-4 font-medium">赔率</th>
                <th className="py-2 pr-4 font-medium">派奖</th>
                <th className="py-2 pr-4 font-medium">状态</th>
              </tr>
            </thead>
            <tbody>
              {settlement.orders.map((order) => (
                <tr key={order.orderId} className="border-b border-slate-100">
                  <td className="py-3 pr-4 font-semibold text-ink">{order.orderId}</td>
                  <td className="py-3 pr-4 text-slate-600">{order.userId}</td>
                  <td className="py-3 pr-4 text-slate-600">{order.ruleCode}</td>
                  <td className="py-3 pr-4">
                    {order.matchedBets.length > 0 ? (
                      <div className="flex max-w-[220px] flex-wrap gap-1">
                        {order.matchedBets.map((bet) => (
                          <Tag key={bet} color="green">
                            {bet}
                          </Tag>
                        ))}
                      </div>
                    ) : (
                      <span className="text-slate-400">未命中</span>
                    )}
                  </td>
                  <td className="py-3 pr-4 text-slate-600">
                    {formatOdds(order.oddsBasisPoints)}
                  </td>
                  <td className="py-3 pr-4 text-slate-600">
                    {formatMoney(order.payoutMinor)}
                  </td>
                  <td className="py-3 pr-4">
                    <Tag color={order.isWinning ? 'green' : 'grey'}>
                      {order.isWinning ? '中奖' : '未中奖'}
                    </Tag>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div className="rounded-md border border-line bg-white p-3 text-sm text-slate-500">
          本期没有待结算订单。
        </div>
      )}
    </div>
  );
}

function InfoLine({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-xs text-slate-400">{label}</div>
      <div className="mt-1 font-medium text-ink">{value}</div>
    </div>
  );
}
