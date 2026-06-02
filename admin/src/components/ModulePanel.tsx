import { Button, Card, Tag } from '@douyinfe/semi-ui';
import { ArrowRight } from 'lucide-react';
import type { ModuleGroup } from '../types/dashboard';

interface ModulePanelProps {
  group: ModuleGroup;
  onOpenModule: (moduleKey: string) => void;
}

export function ModulePanel({ group, onOpenModule }: ModulePanelProps) {
  return (
    <section className="space-y-3">
      <div>
        <h2 className="text-base font-semibold text-ink">{group.title}</h2>
        <p className="mt-1 text-sm text-slate-500">{group.description}</p>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        {group.modules.map((module) => (
          <Card key={module.key} className="rounded-md border border-line">
            <div className="flex min-h-[112px] flex-col justify-between gap-4">
              <div className="space-y-2">
                <div className="flex items-start justify-between gap-3">
                  <h3 className="text-sm font-semibold text-ink">{module.name}</h3>
                  <Tag color={module.status === 'scaffolded' ? 'green' : 'orange'}>
                    {module.status === 'scaffolded' ? '已建入口' : '待实现'}
                  </Tag>
                </div>
                <p className="text-sm leading-5 text-slate-500">{module.description}</p>
              </div>
              <Button
                theme="borderless"
                size="small"
                icon={<ArrowRight size={16} />}
                onClick={() => onOpenModule(module.key)}
              >
                查看
              </Button>
            </div>
          </Card>
        ))}
      </div>
    </section>
  );
}
