import { Fragment } from 'react';
import { getRouteApi } from '@tanstack/react-router';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronRight } from '@fortawesome/free-solid-svg-icons';
import { useTransactions } from './use-transactions';
import { TransactionsToolbar } from './TransactionsToolbar';
import { formatAmount, formatDateHeading } from './format';
import type { Transaction } from './types';
import './TransactionsList.css';

/** Groups transactions by date, assuming they already arrive sorted with same-date rows adjacent. */
const groupByDate = (transactions: Transaction[]): { date: string; transactions: Transaction[] }[] => {
  const groups: { date: string; transactions: Transaction[] }[] = [];
  for (const transaction of transactions) {
    const currentGroup = groups.at(-1);
    if (currentGroup?.date === transaction.date) {
      currentGroup.transactions.push(transaction);
    } else {
      groups.push({ date: transaction.date, transactions: [transaction] });
    }
  }
  return groups;
};

/** A day's total is its net spend: credits (income/transfers) don't offset expenses. */
const dailyTotal = (transactions: Transaction[]): number =>
  transactions
    .filter((transaction) => transaction.category_type === 'expense')
    .reduce((sum, transaction) => sum + Number(transaction.amount), 0);

const TransactionRow = ({ transaction }: { transaction: Transaction }) => {
  const isCredit = transaction.category_type !== 'expense';

  return (
    <li className="transactions-row">
      <div className="transactions-row-cell transactions-row-merchant">{transaction.merchant}</div>
      <div className="transactions-row-cell transactions-row-category">{transaction.category_name_en}</div>
      <div className="transactions-row-cell transactions-row-account">{transaction.account}</div>
      <div className={isCredit ? 'transactions-row-amount transactions-row-amount--credit' : 'transactions-row-amount'}>
        {isCredit ? '+' : ''}
        {formatAmount(Number(transaction.amount))}
      </div>
      <FontAwesomeIcon icon={faChevronRight} className="transactions-row-chevron" />
    </li>
  );
};

const routeApi = getRouteApi('/transactions');

export const TransactionsList = () => {
  const { search, order } = routeApi.useSearch();
  const { data: transactions, isPending, isError } = useTransactions(search, order);

  return (
    <div className="transactions-card">
      <TransactionsToolbar />
      {isPending && <p className="transactions-status">Loading transactions…</p>}
      {isError && <p className="transactions-status">Failed to load transactions.</p>}
      {transactions && transactions.length === 0 && <p className="transactions-status">No transactions yet.</p>}
      {transactions && transactions.length > 0 && (
        <ul className="transactions-rows">
          {groupByDate(transactions).map((group, index) => (
            <Fragment key={`${group.date}-${index}`}>
              <li className="transactions-date-header">
                <span>{formatDateHeading(group.date)}</span>
                <span>{formatAmount(dailyTotal(group.transactions))}</span>
              </li>
              {group.transactions.map((transaction) => (
                <TransactionRow key={transaction.id} transaction={transaction} />
              ))}
            </Fragment>
          ))}
        </ul>
      )}
    </div>
  );
};
