import { createFileRoute, Link } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faEnvelope } from '@fortawesome/free-solid-svg-icons';
import { LanguageSwitcher } from '../components/LanguageSwitcher';
import './index.css';

const LandingPage = () => {
  const { t } = useTranslation();

  return (
    <div className="landing-page">
      <div className="landing-header">
        <div className="landing-mark">
          <img src="/icon-1024x1024.png" alt="" className="landing-mark-icon" />
          <p className="landing-mark-text">
            <span className="landing-mark-accent">Fire</span>
            bird
            <span className="landing-mark-accent">.</span>
          </p>
        </div>
        <LanguageSwitcher />
      </div>
      <div className="landing-hero">
        <h1>{t('landing.headline')}</h1>
        <p className="landing-description">{t('landing.description')}</p>
        <Link to="/sign-in" className="landing-cta">
          <FontAwesomeIcon icon={faEnvelope} />
          {t('landing.signIn')}
        </Link>
      </div>
    </div>
  );
};

export const Route = createFileRoute('/')({
  component: LandingPage,
});
