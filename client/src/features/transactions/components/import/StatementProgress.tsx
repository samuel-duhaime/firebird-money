import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faCheck,
  faFileLines,
  faSpinner,
} from '@fortawesome/free-solid-svg-icons';

/** Where the server is in reading a statement — in the order the steps happen. */
export type ReadStep =
  'uploading' | 'reading' | 'matching' | 'checking_duplicates';

const STEPS = ['reading', 'matching', 'checking_duplicates'] as const;

type StatementProgressProps = {
  fileName: string;
  step: ReadStep;
  /** Lines found so far, while reading. */
  found: number;
  onCancel: () => void;
};

/** The second step: real progress from the server while it reads the statement. */
export const StatementProgress = ({
  fileName,
  step,
  found,
  onCancel,
}: StatementProgressProps) => {
  const { t } = useTranslation();
  // `uploading` comes before every listed step.
  const current = step === 'uploading' ? -1 : STEPS.indexOf(step);

  const detail = (index: number) => {
    if (index < current) {
      return index === 0
        ? t('transactions.import.progress.found', { count: found })
        : t('transactions.import.progress.done');
    }
    if (index > current) return t('transactions.import.progress.waiting');
    if (index === 0 && found > 0) {
      return t('transactions.import.progress.found', { count: found });
    }
    return t(`transactions.import.progress.${STEPS[index]}Active`);
  };

  return (
    <div className="statement-import">
      <div className="statement-file">
        <FontAwesomeIcon
          icon={faFileLines}
          className="statement-file-icon"
          aria-hidden="true"
        />
        <div className="statement-file-text">
          <span className="statement-file-name">{fileName}</span>
          <span className="statement-file-note">
            {t('transactions.import.progress.fileNote')}
          </span>
        </div>
        <button
          type="button"
          className="statement-button statement-button--secondary"
          onClick={onCancel}
        >
          {t('transactions.import.progress.cancel')}
        </button>
      </div>

      <ol className="statement-steps" aria-live="polite">
        {STEPS.map((stepName, index) => {
          const state =
            index < current ? 'done' : index === current ? 'active' : 'waiting';
          return (
            <li
              key={stepName}
              className={`statement-step statement-step--${state}`}
            >
              <span className="statement-step-marker" aria-hidden="true">
                {state === 'done' && <FontAwesomeIcon icon={faCheck} />}
                {state === 'active' && (
                  <FontAwesomeIcon icon={faSpinner} spin />
                )}
              </span>
              <span className="statement-step-text">
                <span className="statement-step-title">
                  {t(`transactions.import.progress.${stepName}`)}
                </span>
                <span className="statement-step-detail">{detail(index)}</span>
              </span>
            </li>
          );
        })}
      </ol>

      <p className="statement-import-hint">
        {t('transactions.import.progress.hint')}
      </p>
    </div>
  );
};
