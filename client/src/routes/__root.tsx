import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { Toaster } from 'sonner';
import './__root.css';

const RootLayout = () => (
  <>
    <Outlet />
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
  </>
);

export const Route = createRootRoute({
  component: RootLayout,
});
