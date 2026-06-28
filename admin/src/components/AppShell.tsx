import { Button } from '@douyinfe/semi-ui';
import {
  Banknote,
  Bot,
  Calculator,
  ChartNoAxesCombined,
  ChevronDown,
  ChevronRight,
  CircleDollarSign,
  ClipboardList,
  Gauge,
  LayoutDashboard,
  LogOut,
  Menu,
  Megaphone,
  Settings,
  ShieldCheck,
  Ticket,
  UserRound,
  UsersRound,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import type { AdminAuthSession } from '../types/auth';

export interface NavigationItem {
  key: string;
  label: string;
  group: string;
}

interface AppShellProps {
  activeKey: string;
  currentSession: AdminAuthSession;
  items: NavigationItem[];
  children: ReactNode;
  onNavigate: (key: string) => void;
  onLogout: () => void;
}

interface NavigationGroup {
  group: string;
  items: NavigationItem[];
}

interface NavigationGroupsProps {
  activeKey: string;
  contentIdPrefix: string;
  expandedGroups: Record<string, boolean>;
  groups: NavigationGroup[];
  onNavigate: (key: string) => void;
  onToggleGroup: (group: string) => void;
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
  advertisements: <Megaphone size={18} />,
  'lottery-console': <Gauge size={18} />,
  lotteries: <Ticket size={18} />,
  'draw-modes': <ChartNoAxesCombined size={18} />,
  schedules: <ChartNoAxesCombined size={18} />,
  'group-buy': <Banknote size={18} />,
  'play-rules': <Calculator size={18} />,
  settlements: <CircleDollarSign size={18} />,
  'group-buy-robot': <Bot size={18} />,
  'purchase-robot': <Bot size={18} />,
  registration: <Settings size={18} />,
  invite: <UsersRound size={18} />,
  rebate: <Banknote size={18} />,
};

const DEFAULT_GROUP_EXPANDED: Record<string, boolean> = {
  常用: true,
  不常用: false,
};

function NavigationGroups({
  activeKey,
  contentIdPrefix,
  expandedGroups,
  groups,
  onNavigate,
  onToggleGroup,
}: NavigationGroupsProps) {
  return (
    <nav className="space-y-5">
      {groups.map(({ group, items: groupItems }, groupIndex) => {
        const isExpanded = expandedGroups[group] ?? true;
        const contentId = `${contentIdPrefix}-${groupIndex}`;

        return (
          <div key={group}>
            <button
              type="button"
              className="mb-2 flex w-full items-center justify-between rounded-md px-2 py-1 text-left text-xs font-semibold text-slate-400 transition hover:bg-slate-50 hover:text-slate-600"
              aria-controls={contentId}
              aria-expanded={isExpanded}
              onClick={() => onToggleGroup(group)}
            >
              <span>{group}</span>
              <span className="grid h-4 w-4 place-items-center">
                {isExpanded ? (
                  <ChevronDown size={14} />
                ) : (
                  <ChevronRight size={14} />
                )}
              </span>
            </button>
            {isExpanded ? (
              <div id={contentId} className="space-y-1">
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
                      <span className="grid h-5 w-5 shrink-0 place-items-center">
                        {iconByKey[item.key] ?? <LayoutDashboard size={18} />}
                      </span>
                      <span className="min-w-0 flex-1 truncate">
                        {item.label}
                      </span>
                    </button>
                  );
                })}
              </div>
            ) : null}
          </div>
        );
      })}
    </nav>
  );
}

export function AppShell({
  activeKey,
  children,
  currentSession,
  items,
  onLogout,
  onNavigate,
}: AppShellProps) {
  const groups = useMemo(
    () =>
      Object.entries(
        items.reduce<Record<string, NavigationItem[]>>((acc, item) => {
          acc[item.group] = [...(acc[item.group] ?? []), item];
          return acc;
        }, {}),
      ).map(([group, groupItems]) => ({
        group,
        items: groupItems,
      })),
    [items],
  );
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>(
    () => ({ ...DEFAULT_GROUP_EXPANDED }),
  );
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const activeItem = useMemo(
    () => items.find((item) => item.key === activeKey),
    [activeKey, items],
  );

  useEffect(() => {
    setExpandedGroups((current) => {
      let changed = false;
      const next = { ...current };

      for (const { group } of groups) {
        if (next[group] === undefined) {
          next[group] = DEFAULT_GROUP_EXPANDED[group] ?? true;
          changed = true;
        }
      }

      return changed ? next : current;
    });
  }, [groups]);

  useEffect(() => {
    if (!mobileNavOpen) {
      return;
    }

    const desktopQuery = window.matchMedia('(min-width: 1024px)');
    const previousOverflow = document.body.style.overflow;
    const closeOnDesktop = () => {
      if (desktopQuery.matches) {
        setMobileNavOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setMobileNavOpen(false);
      }
    };

    document.body.style.overflow = 'hidden';
    document.addEventListener('keydown', closeOnEscape);
    desktopQuery.addEventListener('change', closeOnDesktop);
    closeOnDesktop();

    return () => {
      document.body.style.overflow = previousOverflow;
      document.removeEventListener('keydown', closeOnEscape);
      desktopQuery.removeEventListener('change', closeOnDesktop);
    };
  }, [mobileNavOpen]);

  useEffect(() => {
    const activeGroup = groups.find(({ items: groupItems }) =>
      groupItems.some((item) => item.key === activeKey),
    )?.group;

    if (!activeGroup) {
      return;
    }

    setExpandedGroups((current) =>
      current[activeGroup] ? current : { ...current, [activeGroup]: true },
    );
  }, [activeKey, groups]);

  const toggleGroup = (group: string) => {
    setExpandedGroups((current) => ({
      ...current,
      [group]: !(current[group] ?? true),
    }));
  };

  const handleNavigate = (key: string) => {
    setMobileNavOpen(false);
    onNavigate(key);
  };

  const handleLogout = () => {
    setMobileNavOpen(false);
    onLogout();
  };

  return (
    <div className="flex min-h-screen bg-panel text-ink">
      {mobileNavOpen ? (
        <button
          type="button"
          aria-label="关闭导航菜单"
          className="fixed inset-0 z-40 bg-slate-950/40 lg:hidden"
          onClick={() => setMobileNavOpen(false)}
        />
      ) : null}
      <aside
        className={`fixed inset-y-0 left-0 z-50 w-[min(86vw,20rem)] transform overflow-y-auto border-r border-line bg-white px-4 py-5 shadow-xl transition-transform duration-200 lg:hidden ${
          mobileNavOpen ? 'translate-x-0' : '-translate-x-full'
        }`}
        aria-hidden={!mobileNavOpen}
        aria-label="后台导航"
      >
        <div className="mb-6 flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="truncate text-lg font-semibold">运营工作台</div>
            <div className="truncate text-xs text-slate-500">
              {activeItem?.label ?? '后台管理'}
            </div>
          </div>
          <Button
            aria-label="关闭导航菜单"
            icon={<X size={18} />}
            theme="borderless"
            onClick={() => setMobileNavOpen(false)}
          />
        </div>
        <NavigationGroups
          activeKey={activeKey}
          contentIdPrefix="admin-mobile-nav-group"
          expandedGroups={expandedGroups}
          groups={groups}
          onNavigate={handleNavigate}
          onToggleGroup={toggleGroup}
        />
      </aside>
      <aside className="hidden h-screen w-64 shrink-0 overflow-y-auto border-r border-line bg-white px-4 py-5 lg:sticky lg:top-0 lg:block">
        <div className="mb-6">
          <div className="text-lg font-semibold"></div>
        </div>
        <NavigationGroups
          activeKey={activeKey}
          contentIdPrefix="admin-nav-group"
          expandedGroups={expandedGroups}
          groups={groups}
          onNavigate={handleNavigate}
          onToggleGroup={toggleGroup}
        />
      </aside>
      <main className="min-w-0 flex-1">
        <header className="sticky top-0 z-30 border-b border-line bg-white px-3 py-3 sm:px-6 lg:static lg:py-4">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex min-w-0 items-center gap-3">
              <div className="shrink-0 lg:hidden">
                <Button
                  aria-label="打开导航菜单"
                  icon={<Menu size={18} />}
                  theme="borderless"
                  onClick={() => setMobileNavOpen(true)}
                />
              </div>
              <div className="min-w-0">
                <div className="truncate text-lg font-semibold sm:text-xl">
                  运营工作台
                </div>
                <div className="truncate text-sm text-slate-500">
                  {activeItem?.label ?? '后台管理'}
                </div>
              </div>
            </div>
            <div className="flex min-w-0 items-center justify-between gap-3 text-sm text-slate-500 sm:justify-end">
              <div className="min-w-0 text-left sm:text-right">
                <div className="truncate font-medium text-ink">
                  {currentSession.admin.username}
                </div>
                <div className="truncate text-xs text-slate-500">
                  {currentSession.role.name}
                </div>
              </div>
              <Button
                aria-label="登出"
                theme="borderless"
                icon={<LogOut size={16} />}
                onClick={handleLogout}
              >
                <span className="hidden sm:inline">登出</span>
              </Button>
            </div>
          </div>
        </header>
        <div className="max-w-full px-3 py-4 sm:px-6 lg:px-8">{children}</div>
      </main>
    </div>
  );
}
