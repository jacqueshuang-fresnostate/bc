import { useMemo, useState } from 'react';
import { AppShell, type NavigationItem } from './components/AppShell';
import { useDashboard } from './hooks/useDashboard';
import { DashboardPage } from './pages/DashboardPage';
import { PlaceholderPage } from './pages/PlaceholderPage';

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
