import { getRouteApi } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { SearchButton as SharedSearchButton } from '../../../components/SearchButton';

const routeApi = getRouteApi('/_app/transactions');

/** The transactions page's search, kept in the URL's `search` param. */
export const SearchButton = () => {
  const { t } = useTranslation();
  const { search } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();

  return (
    <SharedSearchButton
      value={search}
      help={t('transactions.search.help')}
      onChange={(term) =>
        navigate({
          search: (prev) => ({ ...prev, search: term }),
          replace: true,
        })
      }
    />
  );
};
