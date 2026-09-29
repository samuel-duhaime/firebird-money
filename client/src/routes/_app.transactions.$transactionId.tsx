import { createFileRoute, useNavigate, useParams } from '@tanstack/react-router';
import { EditTransactionModal } from '../features/transactions/components/EditTransactionModal';

const EditTransactionRoute = () => {
  const { transactionId } = useParams({
    from: '/_app/transactions/$transactionId',
  });
  const navigate = useNavigate();

  return (
    <EditTransactionModal
      transactionId={Number(transactionId)}
      onClose={() =>
        navigate({ to: '/transactions', search: (prev) => prev })
      }
    />
  );
};

export const Route = createFileRoute('/_app/transactions/$transactionId')({
  component: EditTransactionRoute,
});
