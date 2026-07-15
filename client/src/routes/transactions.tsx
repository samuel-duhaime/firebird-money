import { createFileRoute } from '@tanstack/react-router';

const Transactions = () => <h1>Transactions</h1>;

export const Route = createFileRoute('/transactions')({
  component: Transactions,
});
