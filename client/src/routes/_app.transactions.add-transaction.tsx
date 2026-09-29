import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { AddTransactionModal } from '../features/transactions/components/AddTransactionModal';

const AddTransactionRoute = () => {
  const navigate = useNavigate();

  return (
    <AddTransactionModal
      onClose={() => {
        navigate({ to: '/transactions', search: (prev) => prev });
        // The trigger button lives outside this route's Outlet (in the top menu), so it can't be
        // reached via a ref — it's always mounted, so focusing it directly is safe here.
        document.getElementById('add-transaction-button')?.focus();
      }}
    />
  );
};

export const Route = createFileRoute('/_app/transactions/add-transaction')({
  component: AddTransactionRoute,
});
