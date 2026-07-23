import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faSquareCheck, faTableColumns } from '@fortawesome/free-solid-svg-icons';
import { notImplementedToast } from '../../lib/toast';
import './TransactionsToolbar.css';

export const TransactionsToolbar = () => (
  <div className="transactions-toolbar">
    <button type="button" className="transactions-toolbar-button" onClick={notImplementedToast}>
      <span>All transactions</span>
      <FontAwesomeIcon icon={faChevronDown} />
    </button>
    <div className="transactions-toolbar-actions">
      <button type="button" className="transactions-toolbar-button" onClick={notImplementedToast}>
        <FontAwesomeIcon icon={faSquareCheck} />
        <span>Edit multiple</span>
      </button>
      <span className="transactions-toolbar-divider" />
      <button type="button" className="transactions-toolbar-button" onClick={notImplementedToast}>
        <span>Sort</span>
        <FontAwesomeIcon icon={faChevronDown} />
      </button>
      <button type="button" className="transactions-toolbar-button" onClick={notImplementedToast}>
        <FontAwesomeIcon icon={faTableColumns} />
        <span>Columns</span>
      </button>
    </div>
  </div>
);
