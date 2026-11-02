import { useState } from 'react';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { useIsMutating, useMutation } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faCamera,
  faHouse,
  faUser,
  faUserPlus,
} from '@fortawesome/free-solid-svg-icons';
import { submitOnboarding } from '../features/auth/utils/api';
import {
  useCurrentUser,
  useSetCurrentUser,
} from '../features/auth/hooks/use-current-user';
import { requireNotOnboarded } from '../features/auth/utils/require-auth';
import { MemberPicturePicker } from '../features/members/components/MemberPicturePicker';
import { memberPictureMutationKey } from '../features/members/hooks/use-member-picture';
import {
  firstNameRequiredToast,
  joinCodeNotFoundToast,
  onboardingFailedToast,
} from '../lib/toast';
import './auth.css';

type Step = 'name' | 'household' | 'join' | 'picture';

/**
 * Where a signed-in user lands until they've finished onboarding: first their name (what the rest
 * of their household sees), then start a household (and manage it) or join one someone already
 * started, using the code they share. Steps the server says are already done are skipped.
 *
 * Last comes an optional photo. The server doesn't track it as a step: saving a picture needs a
 * household, and leaving this page before choosing one mustn't bring anyone back here.
 */
const OnboardingPage = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: session } = useCurrentUser();
  const setCurrentUser = useSetCurrentUser();
  // Start at the first step still to do — e.g. someone who already has a name skips straight to
  // the household.
  const [step, setStep] = useState<Step>(
    session?.pending_onboarding_steps[0] ?? 'name',
  );
  const [firstName, setFirstName] = useState(session?.user.first_name ?? '');
  const [lastName, setLastName] = useState(session?.user.last_name ?? '');
  const [joinCode, setJoinCode] = useState('');

  const householdPending =
    session?.pending_onboarding_steps.includes('household') ?? true;

  const onboard = useMutation({
    mutationFn: submitOnboarding,
    onSuccess: (newSession) => {
      setCurrentUser(newSession);
      setStep('picture');
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

  const submit = (code?: string) =>
    onboard.mutate({
      firstName: firstName.trim(),
      lastName: lastName.trim(),
      joinCode: code,
    });

  const isSavingPicture =
    useIsMutating({ mutationKey: memberPictureMutationKey }) > 0;

  if (step === 'picture' && session) {
    return (
      <div className="auth-page">
        <div className="auth-card">
          <FontAwesomeIcon icon={faCamera} className="auth-icon" />
          <h1>{t('onboarding.picture.title')}</h1>
          <p className="auth-description">
            {t('onboarding.picture.description')}
          </p>
          <MemberPicturePicker
            memberId={session.user.id}
            name={session.user.first_name ?? session.user.email}
            pictureUrl={session.user.picture_url}
          />
          <div className="auth-form">
            <button
              type="button"
              className="auth-primary"
              disabled={isSavingPicture}
              onClick={() => navigate({ to: '/dashboard' })}
            >
              {session.user.picture_url
                ? t('onboarding.picture.continue')
                : t('onboarding.picture.skip')}
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (step === 'name') {
    return (
      <div className="auth-page">
        <form
          className="auth-card"
          onSubmit={(event) => {
            event.preventDefault();
            // `required` only blocks a truly empty field; whitespace still passes it.
            if (firstName.trim() === '') {
              firstNameRequiredToast();
              return;
            }
            if (householdPending) {
              setStep('household');
              return;
            }
            submit();
          }}
        >
          <FontAwesomeIcon icon={faUser} className="auth-icon" />
          <h1>{t('onboarding.name.title')}</h1>
          <p className="auth-description">{t('onboarding.name.description')}</p>
          <div className="auth-form">
            <input
              type="text"
              required
              autoFocus
              autoComplete="given-name"
              className="auth-input"
              value={firstName}
              placeholder={t('onboarding.name.firstName')}
              aria-label={t('onboarding.name.firstName')}
              onChange={(event) => setFirstName(event.target.value)}
            />
            <input
              type="text"
              autoComplete="family-name"
              className="auth-input"
              value={lastName}
              placeholder={t('onboarding.name.lastName')}
              aria-label={t('onboarding.name.lastName')}
              onChange={(event) => setLastName(event.target.value)}
            />
            <button
              type="submit"
              className="auth-primary"
              disabled={onboard.isPending}
            >
              {t('onboarding.name.submit')}
            </button>
          </div>
        </form>
      </div>
    );
  }

  return (
    <div className="auth-page">
      <div className="auth-card">
        <FontAwesomeIcon
          icon={step === 'join' ? faUserPlus : faHouse}
          className="auth-icon"
        />
        <h1>{t('onboarding.title')}</h1>
        <p className="auth-description">{t('onboarding.description')}</p>

        {step === 'join' ? (
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
              submit(trimmedCode);
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
              onClick={() => setStep('household')}
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
              onClick={() => submit()}
            >
              {t('onboarding.create.submit')}
            </button>
            <button
              type="button"
              className="auth-secondary"
              onClick={() => setStep('join')}
            >
              {t('onboarding.join.trigger')}
            </button>
            <button
              type="button"
              className="auth-secondary"
              onClick={() => setStep('name')}
            >
              {t('onboarding.back')}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export const Route = createFileRoute('/onboarding')({
  beforeLoad: ({ context }) => requireNotOnboarded(context.queryClient),
  component: OnboardingPage,
});
