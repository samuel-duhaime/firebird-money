import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { AddTransactionModal } from '../features/transactions/components/AddTransactionModal';

const AddTransactionRoute = () => {
  const navigate = useNavigate();

  return (
    <AddTransactionModal
      onClose={() =>
        navigate({ to: '/transactions', search: (prev) => prev })
      }
    />
  );
};

export const Route = createFileRoute('/_app/transactions/add-transaction')({
  component: AddTransactionRoute,
});
