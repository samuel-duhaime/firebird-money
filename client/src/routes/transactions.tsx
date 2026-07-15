import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/transactions')({
  component: Transactions,
});

function Transactions() {
  return <h1>Transactions</h1>;
}
