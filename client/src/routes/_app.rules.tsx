import { createFileRoute } from '@tanstack/react-router';

const Rules = () => null;

export const Route = createFileRoute('/_app/rules')({
  component: Rules,
  staticData: { topMenuTitle: 'nav.rules' },
});
