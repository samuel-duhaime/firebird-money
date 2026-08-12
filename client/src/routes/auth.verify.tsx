import { useEffect } from 'react';
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faCircleNotch,
  faTriangleExclamation,
} from '@fortawesome/free-solid-svg-icons';
import { verifyLogin } from '../features/auth/api';
import { useSetCurrentUser } from '../features/auth/use-current-user';
import './auth.css';

/**
 * Where a magic link lands. Spends the token for a session, then sends the user on to onboarding
 * (first login) or the dashboard.
 */
const VerifyPage = () => {
  const { t } = useTranslation();
  const { token } = Route.useSearch();
  const navigate = useNavigate();
  const setCurrentUser = useSetCurrentUser();

  const verification = useQuery({
    queryKey: ['auth', 'verify', token],
    queryFn: () => verifyLogin(token),
    enabled: token !== '',
    // A spent or expired link fails for good; retrying would only spend time before saying so.
    retry: false,
    refetchOnWindowFocus: false,
  });

  const session = verification.data;

  useEffect(() => {
    if (!session) {
      return;
    }

    setCurrentUser(session);
    navigate({
      to: session.households.length > 0 ? '/dashboard' : '/onboarding',
    });
  }, [session, setCurrentUser, navigate]);

  if (token === '' || verification.isError) {
    return (
      <div className="auth-page">
        <div className="auth-card">
          <FontAwesomeIcon icon={faTriangleExclamation} className="auth-icon" />
          <h1>{t('auth.verify.failedTitle')}</h1>
          <p className="auth-description">
            {t('auth.verify.failedDescription')}
          </p>
          <Link to="/sign-in" className="auth-primary">
            {t('auth.verify.tryAgain')}
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="auth-page">
      <div className="auth-card">
        <FontAwesomeIcon icon={faCircleNotch} spin className="auth-icon" />
        <h1>{t('auth.verify.title')}</h1>
      </div>
    </div>
  );
};

export const Route = createFileRoute('/auth/verify')({
  // The token arrives as a query param on the emailed link; anything else is a broken link.
  validateSearch: (search: Record<string, unknown>): { token: string } => ({
    token: typeof search.token === 'string' ? search.token : '',
  }),
  component: VerifyPage,
});
