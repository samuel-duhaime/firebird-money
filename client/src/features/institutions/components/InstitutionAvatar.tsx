import './InstitutionAvatar.css';

type InstitutionAvatarProps = {
  name: string;
};

/** A round first-letter badge, standing in for institution logos until we have real ones. */
export const InstitutionAvatar = ({ name }: InstitutionAvatarProps) => (
  <span className="institution-avatar" aria-hidden="true">
    {name.charAt(0).toUpperCase()}
  </span>
);
