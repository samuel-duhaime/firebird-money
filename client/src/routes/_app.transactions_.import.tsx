import { useCallback, useEffect, useRef, useState } from 'react';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { StatementUpload } from '../features/transactions/components/import/StatementUpload';
import { StatementProgress } from '../features/transactions/components/import/StatementProgress';
import type { ReadStep } from '../features/transactions/components/import/StatementProgress';
import { StatementReview } from '../features/transactions/components/import/StatementReview';
import type { ReviewLine } from '../features/transactions/components/import/StatementReview';
import { useImportTransactions } from '../features/transactions/hooks/use-import-transactions';
import {
  MAX_STATEMENT_BYTES,
  readStatement,
} from '../features/transactions/utils/import';
import type { ImportTransaction } from '../features/transactions/utils/import';
import { importFailedToast } from '../lib/toast';
import '../features/transactions/components/import/StatementImport.css';

type Phase =
  | { kind: 'pick'; error?: string }
  | { kind: 'reading'; fileName: string; step: ReadStep; found: number }
  | { kind: 'review'; fileName: string; lines: ReviewLine[] };

const toImportTransaction = (line: ReviewLine): ImportTransaction => ({
  date: line.date,
  original_statement: line.original_statement,
  amount: line.amount,
  ...(line.merchant_id !== null
    ? { merchant_id: line.merchant_id }
    : { new_merchant_name: line.new_merchant_name ?? line.original_statement }),
  // Only called on lines the Add button allows: a category and an account are settled by then.
  category_id: line.category_id!,
  ...(line.account_id !== null
    ? { account_id: line.account_id }
    : { new_account: line.new_account! }),
  tag_ids: line.tag_ids,
});

// `transactions_` un-nests this page from `/transactions`: it's a page of its own, not a modal
// over the transactions list.
const ImportStatement = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [phase, setPhase] = useState<Phase>({ kind: 'pick' });
  const [accountId, setAccountId] = useState<number | null>(null);
  const readAbort = useRef<AbortController | null>(null);
  const importTransactions = useImportTransactions();
  // `isPending` only re-renders the disabled button a tick later, so a quick double click could
  // send the import twice — and the server adds every line it gets. This blocks it right away.
  const isAdding = useRef(false);

  // Leaving the page mid-read stops the read on the server too, so it stops spending tokens.
  useEffect(() => () => readAbort.current?.abort(), []);

  const startRead = useCallback(
    (file: File) => {
      if (file.size > MAX_STATEMENT_BYTES) {
        setPhase({
          kind: 'pick',
          error: t('transactions.import.errors.tooLarge'),
        });
        return;
      }

      readAbort.current?.abort();
      const controller = new AbortController();
      readAbort.current = controller;
      setPhase({
        kind: 'reading',
        fileName: file.name,
        step: 'uploading',
        found: 0,
      });

      readStatement(
        file,
        accountId,
        (event) =>
          setPhase((previous) => {
            if (previous.kind !== 'reading') return previous;
            if (event.type === 'found')
              return { ...previous, found: event.count };
            if (
              event.type === 'reading' ||
              event.type === 'matching' ||
              event.type === 'checking_duplicates'
            ) {
              return { ...previous, step: event.type };
            }
            return previous;
          }),
        controller.signal,
      )
        .then((transactions) =>
          setPhase({
            kind: 'review',
            fileName: file.name,
            lines: transactions.map((transaction, index) => ({
              ...transaction,
              key: index,
              // A likely duplicate starts unticked.
              selected: transaction.duplicate_of_transaction_id === null,
              useNewAccount: transaction.new_account !== null,
            })),
          }),
        )
        .catch((error: unknown) => {
          if (controller.signal.aborted) return;
          const message = error instanceof Error ? error.message : '';
          setPhase({
            kind: 'pick',
            error: message || t('transactions.import.errors.generic'),
          });
        });
    },
    [accountId, t],
  );

  const cancelRead = () => {
    readAbort.current?.abort();
    setPhase({ kind: 'pick' });
  };

  if (phase.kind === 'reading') {
    return (
      <StatementProgress
        fileName={phase.fileName}
        step={phase.step}
        found={phase.found}
        onCancel={cancelRead}
      />
    );
  }

  if (phase.kind === 'review') {
    return (
      <StatementReview
        fileName={phase.fileName}
        lines={phase.lines}
        onLinesChange={(lines) => setPhase({ ...phase, lines })}
        onDiscard={() => setPhase({ kind: 'pick' })}
        isAdding={importTransactions.isPending}
        onAdd={() => {
          if (isAdding.current) return;
          isAdding.current = true;
          importTransactions.mutate(
            phase.lines
              .filter((line) => line.selected)
              .map(toImportTransaction),
            {
              onSuccess: () => navigate({ to: '/transactions' }),
              onError: importFailedToast,
              onSettled: () => {
                isAdding.current = false;
              },
            },
          );
        }}
      />
    );
  }

  return (
    <StatementUpload
      accountId={accountId}
      onAccountChange={setAccountId}
      onFile={startRead}
      error={phase.error}
    />
  );
};

export const Route = createFileRoute('/_app/transactions_/import')({
  component: ImportStatement,
  staticData: { topMenuTitle: 'transactions.import.title' },
});
