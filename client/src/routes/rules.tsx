import { createFileRoute } from '@tanstack/react-router';

const Rules = () => null;

export const Route = createFileRoute('/rules')({
  component: Rules,
  staticData: { topMenuTitle: 'Rules' },
});
