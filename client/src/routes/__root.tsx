import { createRootRoute, Link, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';

const RootLayout = () => (
  <>
    <nav>
      <Link to="/">Dashboard</Link>
      <Link to="/transactions">Transactions</Link>
    </nav>
    <Outlet />
    <TanStackRouterDevtools />
    <ReactQueryDevtools />
  </>
);

export const Route = createRootRoute({
  component: RootLayout,
});
