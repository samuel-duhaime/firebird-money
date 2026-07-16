import { createFileRoute } from '@tanstack/react-router';
import { useCategories } from '../features/categories/use-categories';

const Dashboard = () => {
  const { data: categories, isPending, isError } = useCategories();

  return (
    <>
      {isPending && <p>Loading categories…</p>}
      {isError && <p>Failed to load categories.</p>}
      {categories && <p>{categories.length} categories loaded from the API.</p>}
    </>
  );
};

export const Route = createFileRoute('/')({
  component: Dashboard,
  staticData: { topMenuTitle: 'Dashboard' },
});
