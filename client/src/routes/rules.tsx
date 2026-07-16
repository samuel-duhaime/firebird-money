import { createFileRoute } from '@tanstack/react-router';

const Rules = () => <h1>Rules</h1>;

export const Route = createFileRoute('/rules')({
  component: Rules,
});
