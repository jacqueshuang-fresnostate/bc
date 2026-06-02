import { useMemo, useState } from 'react';
import { AppShell, type NavigationItem } from './components/AppShell';
import { useDashboard } from './hooks/useDashboard';
import { AccessManagementPage } from './pages/AccessManagementPage';
import { DashboardPage } from './pages/DashboardPage';
import { DrawManagementPage } from './pages/DrawManagementPage';
import { FinanceManagementPage } from './pages/FinanceManagementPage';
import { LotteryManagementPage } from './pages/LotteryManagementPage';
import { OrderManagementPage } from './pages/OrderManagementPage';
import { PlaceholderPage } from './pages/PlaceholderPage';
import { PlayRulesPage } from './pages/PlayRulesPage';
import { SettlementManagementPage } from './pages/SettlementManagementPage';

export function App() {
  const { data, loading, error, refresh } = useDashboard();
  const [activeKey, setActiveKey] = useState('dashboard');

  const navigationItems = useMemo<NavigationItem[]>(() => {
    const moduleItems =
      data?.moduleGroups.flatMap((group) =>
        group.modules.map((module) => ({
          key: module.key,
          label: module.name,
          group: group.title,
        })),
      ) ?? [];

    return [
      {
        key: 'dashboard',
        label: '系统概览',
        group: '工作台',
      },
      ...moduleItems,
    ];
  }, [data]);

  return (
    <AppShell
      activeKey={activeKey}
      items={navigationItems}
      onNavigate={setActiveKey}
    >
      {activeKey === 'dashboard' ? (
        <DashboardPage
          data={data}
          loading={loading}
          error={error}
          onOpenModule={setActiveKey}
          onRefresh={refresh}
        />
      ) : activeKey === 'lotteries' ? (
        <LotteryManagementPage
          onDashboardRefresh={refresh}
          onOpenPlayConfig={() => setActiveKey('play-rules')}
        />
      ) : activeKey === 'draw-modes' || activeKey === 'schedules' ? (
        <DrawManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'orders' ? (
        <OrderManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'finance' ? (
        <FinanceManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'play-rules' ? (
        <PlayRulesPage onDashboardRefresh={refresh} />
      ) : activeKey === 'settlements' ? (
        <SettlementManagementPage onDashboardRefresh={refresh} />
      ) : isAccessModule(activeKey) ? (
        <AccessManagementPage
          activeModuleKey={activeKey}
          onDashboardRefresh={refresh}
        />
      ) : data ? (
        <PlaceholderPage moduleKey={activeKey} summary={data} />
      ) : (
        <DashboardPage
          data={data}
          loading={loading}
          error={error}
          onOpenModule={setActiveKey}
          onRefresh={refresh}
        />
      )}
    </AppShell>
  );
}

function isAccessModule(activeKey: string) {
  return (
    activeKey === 'users' ||
    activeKey === 'admins' ||
    activeKey === 'roles' ||
    activeKey === 'settings' ||
    activeKey === 'registration'
  );
}
