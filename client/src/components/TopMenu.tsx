import type { ComponentType } from 'react';
import { useMatches } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import './TopMenu.css';

declare module '@tanstack/react-router' {
  interface StaticDataRouteOption {
    /** i18n key for the page title rendered in the TopMenu. */
    topMenuTitle?: string;
    /** Renders page-specific buttons on the right side of the TopMenu. */
    topMenuActions?: ComponentType;
  }
}

export const TopMenu = () => {
  const { t } = useTranslation();
  const matches = useMatches();
  // A leaf route with no `staticData` of its own (e.g. a modal/panel route nested under a page)
  // inherits the nearest ancestor's title and actions instead of leaving the top menu blank.
  const activeMatch = [...matches]
    .reverse()
    .find((match) => match.staticData.topMenuTitle);
  const titleKey = activeMatch?.staticData.topMenuTitle;
  const Actions = activeMatch?.staticData.topMenuActions;

  return (
    <header className="top-menu">
      <div className="top-menu-logo">
        <img src="/icon-1024x1024.png" alt="" className="top-menu-logo-icon" />
        <p className="top-menu-logo-text">
          <span className="top-menu-logo-accent">Fire</span>
          bird
          <span className="top-menu-logo-accent">.</span>
        </p>
      </div>
      {titleKey && <h1 className="top-menu-title">{t(titleKey)}</h1>}
      {Actions && (
        <div className="top-menu-actions">
          <Actions />
        </div>
      )}
    </header>
  );
};
