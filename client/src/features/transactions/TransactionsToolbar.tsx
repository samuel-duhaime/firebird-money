import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useTranslation } from 'react-i18next';
import { faChevronDown, faSquareCheck, faTableColumns } from '@fortawesome/free-solid-svg-icons';
import { notImplementedToast } from '../../lib/toast';
import { SortButton } from './SortButton';
import './TransactionsToolbar.css';

export const TransactionsToolbar = () => {
  const { t } = useTranslation();

  return (
    <div className="transactions-toolbar">
      <button type="button" className="transactions-toolbar-button" onClick={notImplementedToast}>
        <span>{t('transactions.toolbar.allTransactions')}</span>
        <FontAwesomeIcon icon={faChevronDown} />
      </button>
      <div className="transactions-toolbar-actions">
        <button type="button" className="transactions-toolbar-button" onClick={notImplementedToast}>
          <FontAwesomeIcon icon={faSquareCheck} />
          <span>{t('transactions.toolbar.editMultiple')}</span>
        </button>
        <span className="transactions-toolbar-divider" />
        <SortButton />
        <button type="button" className="transactions-toolbar-button" onClick={notImplementedToast}>
          <FontAwesomeIcon icon={faTableColumns} />
          <span>{t('transactions.toolbar.columns')}</span>
        </button>
      </div>
    </div>
  );
};
