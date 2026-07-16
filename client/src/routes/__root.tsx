import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { LeftMenu } from '../components/LeftMenu';
import './__root.css';

const RootLayout = () => (
  <div className="app-layout">
    <LeftMenu />
    <main className="app-main">
      <Outlet />
    </main>
    <TanStackRouterDevtools />
    <ReactQueryDevtools />
  </div>
);

export const Route = createRootRoute({
  component: RootLayout,
});
