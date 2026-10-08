import { useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { MerchantModal } from '../features/merchants/components/MerchantModal';
import { MerchantsList } from '../features/merchants/components/MerchantsList';
import type { Merchant } from '../features/merchants/utils/types';
import './_app.settings.css';

const MerchantsPage = () => {
  const { t } = useTranslation();
  const [modalMerchant, setModalMerchant] = useState<Merchant | 'new' | null>(
    null,
  );

  return (
    <div className="settings-panel">
      <div className="settings-panel-header">
        <h2>{t('settings.merchants.heading')}</h2>
        <button
          type="button"
          className="settings-panel-primary-button"
          onClick={() => setModalMerchant('new')}
        >
          {t('settings.merchants.newMerchant')}
        </button>
      </div>

      <MerchantsList onEdit={setModalMerchant} />

      {modalMerchant && (
        <MerchantModal
          merchant={modalMerchant === 'new' ? undefined : modalMerchant}
          onClose={() => setModalMerchant(null)}
        />
      )}
    </div>
  );
};

export const Route = createFileRoute('/_app/settings/merchants')({
  component: MerchantsPage,
});
