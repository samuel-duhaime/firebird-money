import { useState } from 'react';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { useMutation } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faPaperPlane } from '@fortawesome/free-solid-svg-icons';
import { requestLogin } from '../features/auth/api';
import { useSetCurrentUser } from '../features/auth/use-current-user';
import { signInFailedToast } from '../lib/toast';
import './auth.css';

/**
 * Passwordless sign-in: type an email, get a link. There is no password and no separate sign-up —
 * an unknown address simply becomes an account.
 */
const SignInPage = () => {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const setCurrentUser = useSetCurrentUser();
  const [email, setEmail] = useState('');
  const [linkSent, setLinkSent] = useState(false);

  const signIn = useMutation({
    mutationFn: (address: string) =>
      requestLogin(address, i18n.resolvedLanguage ?? i18n.language),
    onSuccess: (response) => {
      // The server skipped the email (localhost without a mail provider) and signed us in.
      if (response.status === 'signed_in') {
        setCurrentUser(response.session);
        navigate({
          to:
            response.session.households.length > 0
              ? '/dashboard'
              : '/onboarding',
        });
        return;
      }

      setLinkSent(true);
    },
    onError: signInFailedToast,
  });

  if (linkSent) {
    return (
      <div className="auth-page">
        <div className="auth-card">
          <FontAwesomeIcon icon={faPaperPlane} className="auth-icon" />
          <h1>{t('auth.checkInbox.title')}</h1>
          <p className="auth-description">
            {t('auth.checkInbox.description', { email })}
          </p>
          <button
            type="button"
            className="auth-secondary"
            onClick={() => setLinkSent(false)}
          >
            {t('auth.checkInbox.useAnotherEmail')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="auth-page">
      <form
        className="auth-card"
        onSubmit={(event) => {
          event.preventDefault();
          signIn.mutate(email);
        }}
      >
        <h1>{t('auth.signIn.title')}</h1>
        <p className="auth-description">{t('auth.signIn.description')}</p>
        <input
          type="email"
          required
          autoFocus
          className="auth-input"
          value={email}
          placeholder={t('auth.signIn.placeholder')}
          onChange={(event) => setEmail(event.target.value)}
        />
        <button
          type="submit"
          className="auth-primary"
          disabled={signIn.isPending}
        >
          {signIn.isPending
            ? t('auth.signIn.sending')
            : t('auth.signIn.submit')}
        </button>
      </form>
    </div>
  );
};

export const Route = createFileRoute('/sign-in')({
  component: SignInPage,
});
