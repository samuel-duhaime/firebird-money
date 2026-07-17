import type { ComponentType } from 'react';
import { useMatches } from '@tanstack/react-router';
import './TopMenu.css';

declare module '@tanstack/react-router' {
  interface StaticDataRouteOption {
    /** Page title rendered in the TopMenu. */
    topMenuTitle?: string;
    /** Renders page-specific buttons on the right side of the TopMenu. */
    topMenuActions?: ComponentType;
  }
}

export const TopMenu = () => {
  const matches = useMatches();
  const leafMatch = matches.at(-1);
  const title = leafMatch?.staticData.topMenuTitle;
  const Actions = leafMatch?.staticData.topMenuActions;

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
      {title && <h1 className="top-menu-title">{title}</h1>}
      {Actions && (
        <div className="top-menu-actions">
          <Actions />
        </div>
      )}
    </header>
  );
};
