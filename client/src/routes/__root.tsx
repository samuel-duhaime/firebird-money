import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { Toaster } from 'sonner';
import { TopMenu } from '../components/TopMenu';
import { LeftMenu } from '../components/LeftMenu';
import './__root.css';

const RootLayout = () => (
  <div className="app-layout">
    <TopMenu />
    <div className="app-body">
      <LeftMenu />
      <main className="app-main">
        <Outlet />
      </main>
    </div>
    <Toaster
      theme="dark"
      position="bottom-right"
      toastOptions={{
        style: {
          background: 'var(--color-dark)',
          border: '1px solid var(--color-medium)',
          color: 'var(--color-white)',
          font: '400 15px/1.5 Quicksand, sans-serif',
        },
      }}
    />
    <TanStackRouterDevtools />
    <ReactQueryDevtools />
  </div>
);

export const Route = createRootRoute({
  component: RootLayout,
});
