import { Input, Banner, Button, Card } from '@douyinfe/semi-ui';
import { LogIn } from 'lucide-react';
import { useState } from 'react';
import type { AdminLoginRequest } from '../types/auth';

interface LoginPageProps {
  error: string | null;
  loading: boolean;
  onLogin: (payload: AdminLoginRequest) => Promise<unknown>;
}

export function LoginPage({ error, loading, onLogin }: LoginPageProps) {
  const [username, setUsername] = useState('admin');
  const [password, setPassword] = useState('');

  const submit = async () => {
    await onLogin({
      password: password.trim(),
      username: username.trim(),
    });
  };

  return (
    <div className="grid min-h-screen place-items-center bg-panel px-4 py-8 text-ink">
      <Card className="w-full max-w-[420px] rounded-md border border-line">
        <div className="mb-5">
          <h1 className="text-xl font-semibold text-ink">彩票管理后台</h1>
          <p className="mt-1 text-sm text-slate-500">管理员登录</p>
        </div>

        {error ? (
          <Banner className="mb-4" type="danger" title="登录失败" description={error} />
        ) : null}

        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <label className="block space-y-1">
            <span className="text-xs font-medium text-slate-500">账号</span>
            <Input
              className="form-input"
              autoComplete="username"
              value={username}
              onChange={(value) => setUsername(value)}
            />
          </label>
          <label className="block space-y-1">
            <span className="text-xs font-medium text-slate-500">密码</span>
            <Input
              className="form-input"
              autoComplete="current-password"
              type="password"
              value={password}
              onChange={(value) => setPassword(value)}
            />
          </label>
          <Button
            className="w-full"
            disabled={!username.trim() || !password.trim()}
            icon={<LogIn size={16} />}
            htmlType="submit"
            loading={loading}
            theme="solid"
          >
            登录
          </Button>
        </form>
      </Card>
    </div>
  );
}
