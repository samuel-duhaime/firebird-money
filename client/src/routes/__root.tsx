import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
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
    <TanStackRouterDevtools />
    <ReactQueryDevtools />
  </div>
);

export const Route = createRootRoute({
  component: RootLayout,
});
