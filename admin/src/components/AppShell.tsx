import {
  Banknote,
  Bot,
  Calculator,
  ChartNoAxesCombined,
  CircleDollarSign,
  ClipboardList,
  LayoutDashboard,
  Settings,
  ShieldCheck,
  Ticket,
  UserRound,
  UsersRound,
} from 'lucide-react';
import type { ReactNode } from 'react';

export interface NavigationItem {
  key: string;
  label: string;
  group: string;
}

interface AppShellProps {
  activeKey: string;
  items: NavigationItem[];
  children: ReactNode;
  onNavigate: (key: string) => void;
}

const iconByKey: Record<string, ReactNode> = {
  dashboard: <LayoutDashboard size={18} />,
  users: <UserRound size={18} />,
  orders: <ClipboardList size={18} />,
  finance: <CircleDollarSign size={18} />,
  support: <UsersRound size={18} />,
  admins: <ShieldCheck size={18} />,
  roles: <ShieldCheck size={18} />,
  settings: <Settings size={18} />,
  lotteries: <Ticket size={18} />,
  'draw-modes': <ChartNoAxesCombined size={18} />,
  schedules: <ChartNoAxesCombined size={18} />,
  'group-buy': <Banknote size={18} />,
  'play-rules': <Calculator size={18} />,
  'group-buy-robot': <Bot size={18} />,
  'purchase-robot': <Bot size={18} />,
  registration: <UserRound size={18} />,
  invite: <UsersRound size={18} />,
  rebate: <Banknote size={18} />,
};

export function AppShell({ activeKey, items, children, onNavigate }: AppShellProps) {
  const groups = items.reduce<Record<string, NavigationItem[]>>((acc, item) => {
    acc[item.group] = [...(acc[item.group] ?? []), item];
    return acc;
  }, {});

  return (
    <div className="flex min-h-screen bg-panel text-ink">
      <aside className="hidden w-64 shrink-0 border-r border-line bg-white px-4 py-5 lg:block">
        <div className="mb-6">
          <div className="text-lg font-semibold">彩票管理后台</div>
          <div className="mt-1 text-xs text-slate-500">Admin Console</div>
        </div>
        <nav className="space-y-5">
          {Object.entries(groups).map(([group, groupItems]) => (
            <div key={group}>
              <div className="mb-2 px-2 text-xs font-semibold text-slate-400">
                {group}
              </div>
              <div className="space-y-1">
                {groupItems.map((item) => {
                  const isActive = activeKey === item.key;
                  return (
                    <button
                      key={item.key}
                      className={`flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition ${
                        isActive
                          ? 'bg-teal-50 font-semibold text-accent'
                          : 'text-slate-600 hover:bg-slate-100 hover:text-ink'
                      }`}
                      type="button"
                      onClick={() => onNavigate(item.key)}
                    >
                      <span className="grid h-5 w-5 place-items-center">
                        {iconByKey[item.key] ?? <LayoutDashboard size={18} />}
                      </span>
                      <span className="min-w-0 flex-1 truncate">{item.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </nav>
      </aside>
      <main className="min-w-0 flex-1">
        <header className="border-b border-line bg-white px-4 py-4 sm:px-6">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <div className="text-xl font-semibold">运营工作台</div>
              <div className="text-sm text-slate-500">
                首期骨架已接入后端概览接口
              </div>
            </div>
            <div className="text-sm text-slate-500">Asia/Hong_Kong</div>
          </div>
        </header>
        <div className="px-4 py-5 sm:px-6">{children}</div>
      </main>
    </div>
  );
}
