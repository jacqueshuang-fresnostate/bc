import { Tag } from '@douyinfe/semi-ui';
import type { OrderDetail } from '../types/orders';
import { formatPlaySelection } from '../utils/orderBetInfo';

interface OrderBetInfoProps {
  compact?: boolean;
  expandedLimit?: number;
  order: Pick<OrderDetail, 'expandedBets' | 'matchedBets' | 'selection'>;
  showExpandedBets?: boolean;
}

/** 展示后台订单的用户下注选择和展开后的注码，供订单管理与控单页面复用。 */
export function OrderBetInfo({
  compact = false,
  expandedLimit = compact ? 6 : 10,
  order,
  showExpandedBets = true,
}: OrderBetInfoProps) {
  const selectionLines = formatPlaySelection(order.selection);
  const expandedBets = order.expandedBets ?? [];
  const matchedBets = new Set(order.matchedBets ?? []);
  const visibleExpandedBets = expandedBets.slice(0, expandedLimit);
  const hiddenCount = Math.max(0, expandedBets.length - visibleExpandedBets.length);

  return (
    <div className={compact ? 'min-w-[220px]' : 'min-w-[280px]'}>
      <div className="space-y-1">
        {selectionLines.map((line) => (
          <div
            key={`${line.label}-${line.value}`}
            className="grid grid-cols-[52px_1fr] gap-2 text-xs leading-5"
          >
            <span className="text-slate-400">{line.label}</span>
            <span className="min-w-0 break-words font-medium text-slate-700">
              {line.value}
            </span>
          </div>
        ))}
      </div>

      {showExpandedBets ? (
        <div
          className="mt-2 flex max-w-[320px] flex-wrap gap-1"
          title={expandedBets.length > 0 ? expandedBets.join('、') : undefined}
        >
          {visibleExpandedBets.length > 0 ? (
            visibleExpandedBets.map((bet, index) => (
              <Tag
                key={`${bet}-${index}`}
                color={matchedBets.has(bet) ? 'green' : 'blue'}
              >
                {bet}
              </Tag>
            ))
          ) : (
            <span className="text-xs text-slate-400">暂无展开注码</span>
          )}
          {hiddenCount > 0 ? <Tag color="grey">+{hiddenCount}</Tag> : null}
        </div>
      ) : null}
    </div>
  );
}
