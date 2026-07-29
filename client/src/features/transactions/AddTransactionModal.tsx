import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useCategories } from '../categories/use-categories';
import './AddTransactionModal.css';

type AddTransactionModalProps = {
  open: boolean;
  onClose: () => void;
};

export function AddTransactionModal({
  open,
  onClose,
}: AddTransactionModalProps) {
  const { t, i18n } = useTranslation();
  const { data: categories } = useCategories();
  const [amount, setAmount] = useState('');
  const [merchant, setMerchant] = useState('');
  const [date, setDate] = useState('');
  const [category, setCategory] = useState('');

  if (!open) return null;

  const handleSubmit = () => {
    // API integration will be added in a later issue.
    console.log({
      amount,
      merchant,
      date,
      category,
    });

    onClose();
  };

  return (
    <div className="modal-overlay">
      <div className="add-transaction-modal">
        <div className="modal-header">
          <h2>{t('transactions.add.title', 'Add transaction')}</h2>

          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label="Close"
          >
            ×
          </button>
        </div>

        <div className="modal-body">
          <label htmlFor="amount">
            {t('transactions.add.amount', 'Amount')}
          </label>
          <input
            id="amount"
            type="number"
            min="0"
            step="0.01"
            placeholder="$0.00"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
/>
          <label htmlFor="merchant ">
            {t('transactions.add.merchant', 'Merchant')}
          </label>
          <input
            id="merchant"
            type="text"
            placeholder="Merchant Name"
            value={merchant}
            onChange={(e) => setMerchant(e.target.value)}
          />

          <label htmlFor="date">
            {t('transactions.add.date', 'Date')}
          </label>
          <input
            id="date"
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
          />

          <label htmlFor="category">
            {t('transactions.add.category', 'Category')}
          </label>
          <select
            id="category"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          >
            <option value="">
              {t('transactions.add.selectCategory', 'Select a category')}
            </option>

            {categories?.map((category) => (
            <option key={category.id} value={category.id}>
            {i18n.language === 'fr' ? category.name_fr : category.name_en}
            
            </option>
            ))}
          </select>
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="modal-button modal-button--cancel"
            onClick={onClose}
          >
            {t('common.cancel', 'Cancel')}
          </button>

          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSubmit}
          >
            {t('transactions.add.submit', 'Add transaction')}
          </button>
        </div>
      </div>
    </div>
  );
}