import { useEffect, useMemo, useState } from 'react';
import { AppShell, type NavigationItem } from './components/AppShell';
import { useAuth } from './hooks/useAuth';
import { useDashboard } from './hooks/useDashboard';
import { AccessManagementPage } from './pages/AccessManagementPage';
import { DashboardPage } from './pages/DashboardPage';
import { DrawManagementPage } from './pages/DrawManagementPage';
import { FinanceManagementPage } from './pages/FinanceManagementPage';
import { GroupBuyManagementPage } from './pages/GroupBuyManagementPage';
import { InviteManagementPage } from './pages/InviteManagementPage';
import { LotteryConsolePage } from './pages/LotteryConsolePage';
import { LotteryManagementPage } from './pages/LotteryManagementPage';
import { LoginPage } from './pages/LoginPage';
import { OrderManagementPage } from './pages/OrderManagementPage';
import { PlaceholderPage } from './pages/PlaceholderPage';
import { PlayRulesPage } from './pages/PlayRulesPage';
import { RebateManagementPage } from './pages/RebateManagementPage';
import { RobotManagementPage } from './pages/RobotManagementPage';
import { SettlementManagementPage } from './pages/SettlementManagementPage';
import { SupportManagementPage } from './pages/SupportManagementPage';
import type { DashboardSummary, PermissionScope } from './types/dashboard';

export function App() {
  const {
    error: authError,
    loading: authLoading,
    login,
    logout,
    saving: authSaving,
    session,
  } = useAuth();
  const { data, loading, error, refresh } = useDashboard(Boolean(session));
  const [activeKey, setActiveKey] = useState('dashboard');
  const filteredData = useMemo(
    () => (data && session ? filterDashboardByScopes(data, session.scopes) : data),
    [data, session],
  );

  const navigationItems = useMemo<NavigationItem[]>(() => {
    const moduleItems =
      filteredData?.moduleGroups.flatMap((group) =>
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
  }, [filteredData]);

  useEffect(() => {
    if (!navigationItems.some((item) => item.key === activeKey)) {
      setActiveKey('dashboard');
    }
  }, [activeKey, navigationItems]);

  if (authLoading) {
    return (
      <div className="grid min-h-screen place-items-center bg-panel text-sm text-slate-500">
        正在校验登录状态
      </div>
    );
  }

  if (!session) {
    return (
      <LoginPage
        error={authError}
        loading={authSaving}
        onLogin={login}
      />
    );
  }

  return (
    <AppShell
      activeKey={activeKey}
      currentSession={session}
      items={navigationItems}
      onLogout={() => void logout()}
      onNavigate={setActiveKey}
    >
      {activeKey === 'dashboard' ? (
        <DashboardPage
          data={filteredData}
          loading={loading}
          error={error}
          onOpenModule={setActiveKey}
          onRefresh={refresh}
        />
      ) : activeKey === 'lotteries' ? (
        <LotteryManagementPage
          settings={filteredData?.settings ?? []}
          onDashboardRefresh={refresh}
          onOpenPlayConfig={() => setActiveKey('play-rules')}
        />
      ) : activeKey === 'lottery-console' ? (
        <LotteryConsolePage onDashboardRefresh={refresh} />
      ) : activeKey === 'draw-modes' || activeKey === 'schedules' ? (
        <DrawManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'orders' ? (
        <OrderManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'finance' ? (
        <FinanceManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'support' ? (
        <SupportManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'group-buy' ? (
        <GroupBuyManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'invite' ? (
        <InviteManagementPage onDashboardRefresh={refresh} />
      ) : activeKey === 'play-rules' ? (
        <PlayRulesPage onDashboardRefresh={refresh} />
      ) : activeKey === 'settlements' ? (
        <SettlementManagementPage onDashboardRefresh={refresh} />
      ) : isAccessModule(activeKey) ? (
        <AccessManagementPage
          activeModuleKey={activeKey}
          onDashboardRefresh={refresh}
        />
      ) : isRobotModule(activeKey) ? (
        <RobotManagementPage
          activeModuleKey={activeKey}
          onDashboardRefresh={refresh}
        />
      ) : activeKey === 'rebate' ? (
        <RebateManagementPage onDashboardRefresh={refresh} />
      ) : filteredData ? (
        <PlaceholderPage moduleKey={activeKey} summary={filteredData} />
      ) : (
        <DashboardPage
          data={filteredData}
          loading={loading}
          error={error}
          onOpenModule={setActiveKey}
          onRefresh={refresh}
        />
      )}
    </AppShell>
  );
}

function filterDashboardByScopes(
  data: DashboardSummary,
  scopes: PermissionScope[],
): DashboardSummary {
  return {
    ...data,
    moduleGroups: data.moduleGroups
      .map((group) => ({
        ...group,
        modules: group.modules.filter((module) =>
          moduleAllowed(module.key, scopes),
        ),
      }))
      .filter((group) => group.modules.length > 0),
  };
}

function moduleAllowed(moduleKey: string, scopes: PermissionScope[]) {
  const requiredScope = scopeForModule(moduleKey);
  return !requiredScope || scopes.includes(requiredScope);
}

function scopeForModule(moduleKey: string): PermissionScope | null {
  const moduleScopeMap: Record<string, PermissionScope> = {
    admins: 'admins',
    finance: 'finance',
    invite: 'rebates',
    orders: 'orders',
    rebate: 'rebates',
    registration: 'users',
    roles: 'roles',
    settings: 'systemSettings',
    support: 'customerService',
    users: 'users',
  };

  if (moduleKey in moduleScopeMap) {
    return moduleScopeMap[moduleKey];
  }
  if (moduleKey === 'group-buy-robot' || moduleKey === 'purchase-robot') {
    return 'robots';
  }
  if (
    moduleKey === 'lottery-console' ||
    moduleKey === 'lotteries' ||
    moduleKey === 'draw-modes' ||
    moduleKey === 'schedules' ||
    moduleKey === 'group-buy' ||
    moduleKey === 'play-rules' ||
    moduleKey === 'settlements'
  ) {
    return moduleKey === 'settlements' ? 'orders' : 'lotteries';
  }

  return null;
}

function isRobotModule(activeKey: string) {
  return activeKey === 'group-buy-robot' || activeKey === 'purchase-robot';
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
