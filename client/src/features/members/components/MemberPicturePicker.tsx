import { useRef } from 'react';
import type { ChangeEvent } from 'react';
import { useIsMutating } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Avatar } from '../../../components/Avatar';
import {
  memberPictureMutationKey,
  useDeleteMemberPicture,
  useUploadMemberPicture,
} from '../hooks/use-member-picture';
import { pictureSrc } from '../utils/format';
import './MemberPicturePicker.css';

type MemberPicturePickerProps = {
  memberId: number;
  name: string;
  pictureUrl: string | null;
};

/** Shows a member's picture and lets it be chosen, replaced, or removed. Each change saves at once. */
export const MemberPicturePicker = ({
  memberId,
  name,
  pictureUrl,
}: MemberPicturePickerProps) => {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  const upload = useUploadMemberPicture();
  const remove = useDeleteMemberPicture();
  // Any picture change, not just this picker's: one still running after the editor was closed and
  // reopened could otherwise finish after a newer choice and overwrite it.
  const isPending =
    useIsMutating({ mutationKey: memberPictureMutationKey }) > 0;

  const handleFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    // Cleared so choosing the same file again, e.g. after a failed upload, still fires a change.
    event.target.value = '';
    if (file) upload.mutate({ id: memberId, file });
  };

  return (
    <div className="member-picture-picker">
      <Avatar name={name} src={pictureSrc(pictureUrl)} size="large" />
      <input
        ref={inputRef}
        type="file"
        accept="image/png,image/jpeg,image/webp"
        className="member-picture-picker-input"
        aria-label={t('settings.members.picture.choose')}
        tabIndex={-1}
        onChange={handleFileChange}
      />
      <div className="member-picture-picker-actions">
        <button
          type="button"
          className="member-picture-picker-button"
          disabled={isPending}
          onClick={() => inputRef.current?.click()}
        >
          {t('settings.members.picture.choose')}
        </button>
        {pictureUrl && (
          <button
            type="button"
            className="member-picture-picker-button"
            disabled={isPending}
            onClick={() => remove.mutate(memberId)}
          >
            {t('settings.members.picture.remove')}
          </button>
        )}
      </div>
    </div>
  );
};
