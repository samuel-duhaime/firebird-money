import { useState } from 'react';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { useMutation } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faHouse, faUserPlus } from '@fortawesome/free-solid-svg-icons';
import { submitOnboarding } from '../features/auth/api';
import { useSetCurrentUser } from '../features/auth/use-current-user';
import { joinCodeNotFoundToast, onboardingFailedToast } from '../lib/toast';
import './auth.css';

/**
 * What a first login lands on: start a household (and manage it) or join one someone already
 * started, using the code they share.
 */
const OnboardingPage = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const setCurrentUser = useSetCurrentUser();
  const [joining, setJoining] = useState(false);
  const [joinCode, setJoinCode] = useState('');

  const onboard = useMutation({
    mutationFn: submitOnboarding,
    onSuccess: (session) => {
      setCurrentUser(session);
      navigate({ to: '/dashboard' });
    },
    onError: (error: Error) => {
      // The API answers 404 when no household carries that code — worth saying precisely, since
      // a mistyped code is the likely cause.
      if (error.message.includes('404')) {
        joinCodeNotFoundToast();
        return;
      }
      onboardingFailedToast();
    },
  });

  return (
    <div className="auth-page">
      <div className="auth-card">
        <FontAwesomeIcon
          icon={joining ? faUserPlus : faHouse}
          className="auth-icon"
        />
        <h1>{t('onboarding.title')}</h1>
        <p className="auth-description">{t('onboarding.description')}</p>

        {joining ? (
          <form
            className="auth-form"
            onSubmit={(event) => {
              event.preventDefault();
              // `required` only blocks a truly empty field; whitespace still passes it.
              const trimmedCode = joinCode.trim();
              if (trimmedCode === '') {
                joinCodeNotFoundToast();
                return;
              }
              onboard.mutate(trimmedCode);
            }}
          >
            <input
              type="text"
              required
              autoFocus
              className="auth-input"
              value={joinCode}
              placeholder={t('onboarding.join.placeholder')}
              onChange={(event) => setJoinCode(event.target.value)}
            />
            <button
              type="submit"
              className="auth-primary"
              disabled={onboard.isPending}
            >
              {t('onboarding.join.submit')}
            </button>
            <button
              type="button"
              className="auth-secondary"
              onClick={() => setJoining(false)}
            >
              {t('onboarding.back')}
            </button>
          </form>
        ) : (
          <div className="auth-form">
            <button
              type="button"
              className="auth-primary"
              disabled={onboard.isPending}
              onClick={() => onboard.mutate(undefined)}
            >
              {t('onboarding.create.submit')}
            </button>
            <button
              type="button"
              className="auth-secondary"
              onClick={() => setJoining(true)}
            >
              {t('onboarding.join.trigger')}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export const Route = createFileRoute('/onboarding')({
  component: OnboardingPage,
});
