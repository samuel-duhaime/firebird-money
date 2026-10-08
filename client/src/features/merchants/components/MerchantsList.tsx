import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ApiError } from '../../../lib/api-client';
import { useMerchants } from '../hooks/use-merchants';
import { useDeleteMerchant } from '../hooks/use-delete-merchant';
import {
  deleteMerchantFailedToast,
  deleteMerchantInUseToast,
} from '../../../lib/toast';
import type { Merchant, MerchantSortOrder } from '../utils/types';
import './MerchantsList.css';

type MerchantsListProps = {
  onEdit: (merchant: Merchant) => void;
};

export const MerchantsList = ({ onEdit }: MerchantsListProps) => {
  const { t } = useTranslation();
  const [order, setOrder] = useState<MerchantSortOrder>('transaction_count');
  const [search, setSearch] = useState('');
  const { data: merchants, isPending, isError } = useMerchants(order);
  const deleteMerchantMutation = useDeleteMerchant();
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(
    null,
  );

  const total = merchants?.length ?? 0;
  const filtered = useMemo(() => {
    const term = search.trim().toLowerCase();
    if (!term) return merchants ?? [];
    return (merchants ?? []).filter((merchant) =>
      merchant.name.toLowerCase().includes(term),
    );
  }, [merchants, search]);

  const handleDelete = (id: number) => {
    deleteMerchantMutation.mutate(id, {
      onSuccess: () => setConfirmingDeleteId(null),
      onError: (error) => {
        // 409 here is specifically "still used by existing transactions" (merchant-in-use, see
        // merchants::handlers::delete_merchant) — worth a distinct, actionable message instead of
        // the generic failure toast every other delete error gets.
        if (error instanceof ApiError && error.status === 409) {
          deleteMerchantInUseToast();
        } else {
          deleteMerchantFailedToast();
        }
      },
    });
  };

  return (
    <div className="merchants-list-wrapper">
      <div className="merchants-list-toolbar">
        <input
          type="text"
          className="merchants-list-search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t('settings.merchants.search', { count: total })}
        />

        <select
          className="merchants-list-sort"
          aria-label={t('settings.merchants.sort.label')}
          value={order}
          onChange={(e) => setOrder(e.target.value as MerchantSortOrder)}
        >
          <option value="transaction_count">
            {t('settings.merchants.sort.transactionCount')}
          </option>
          <option value="alphabetical">
            {t('settings.merchants.sort.alphabetical')}
          </option>
        </select>
      </div>

      {!isPending && !isError && (
        <p className="merchants-list-total">
          {t('settings.merchants.total', { count: total })}
        </p>
      )}

      {isPending && <p>{t('settings.merchants.loading')}</p>}
      {isError && <p>{t('settings.merchants.error')}</p>}
      {!isPending && !isError && filtered.length === 0 && (
        <p>{t('settings.merchants.empty')}</p>
      )}

      {!isPending && !isError && filtered.length > 0 && (
        <ul className="merchants-list">
          {filtered.map((merchant) => {
            // Common merchants (shared across every household) can't be renamed/deleted through
            // this API — see merchants::repository::update/delete — so there's nothing useful an
            // Edit/Delete button could do here; hiding them is cleaner than a 404 on click.
            const isCustom = merchant.household_id !== null;

            return (
              <li key={merchant.id} className="merchants-list-row">
                <span className="merchants-list-avatar" aria-hidden="true">
                  {merchant.name.charAt(0).toUpperCase()}
                </span>
                <span className="merchants-list-name">{merchant.name}</span>
                <span className="merchants-list-meta">
                  {t('settings.merchants.transactionCount', {
                    count: merchant.transaction_count,
                  })}
                </span>

                {isCustom &&
                  (confirmingDeleteId === merchant.id ? (
                    <div className="merchants-list-confirm">
                      <span className="merchants-list-confirm-text">
                        {t('settings.merchants.deleteConfirm', {
                          name: merchant.name,
                        })}
                      </span>
                      <div className="merchants-list-actions">
                        <button
                          type="button"
                          className="merchants-list-action"
                          onClick={() => setConfirmingDeleteId(null)}
                        >
                          {t('settings.merchants.modal.cancel')}
                        </button>
                        <button
                          type="button"
                          className="merchants-list-action merchants-list-action--danger"
                          onClick={() => handleDelete(merchant.id)}
                          disabled={deleteMerchantMutation.isPending}
                        >
                          {t('settings.merchants.deleteConfirmButton')}
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="merchants-list-actions">
                      <button
                        type="button"
                        className="merchants-list-action"
                        onClick={() => onEdit(merchant)}
                      >
                        {t('settings.merchants.edit')}
                      </button>
                      <button
                        type="button"
                        className="merchants-list-action merchants-list-action--danger"
                        onClick={() => setConfirmingDeleteId(merchant.id)}
                      >
                        {t('settings.merchants.delete')}
                      </button>
                    </div>
                  ))}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
};
