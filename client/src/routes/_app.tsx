import { createFileRoute, Outlet } from '@tanstack/react-router';
import { TopMenu } from '../components/TopMenu';
import { LeftMenu } from '../components/LeftMenu';
import { requireAuth } from '../features/auth/require-auth';
import './_app.css';

const AppLayout = () => (
  <div className="app-layout">
    <TopMenu />
    <div className="app-body">
      <LeftMenu />
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  </div>
);

export const Route = createFileRoute('/_app')({
  beforeLoad: ({ context }) => requireAuth(context.queryClient),
  component: AppLayout,
});
