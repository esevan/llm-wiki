import type { MigrationRecovery } from "../../services/vaultSetupClient";

interface Props {
  recovery: MigrationRecovery;
  busy: boolean;
  error: string;
  onRestore(): void;
  onRetry(): void;
}

export function MigrationRecoveryView({
  recovery,
  busy,
  error,
  onRestore,
  onRetry,
}: Props) {
  return (
    <div className="vault-setup-backdrop" role="presentation">
      <section
        className="vault-setup-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby="migration-recovery-title"
      >
        <div className="vault-setup-mark" aria-hidden="true">
          !
        </div>
        <p className="vault-setup-eyebrow">LOCAL DATABASE RECOVERY</p>
        <h1 id="migration-recovery-title">
          Recovery is needed before work can continue
        </h1>
        <p>
          The last local database migration did not finish. Your work is
          protected while you choose a recovery action.
        </p>
        <p className="vault-setup-detail">
          No Vault files or work records are changed automatically. Restore uses
          the verified local backup; retry attempts the migration again.
        </p>
        {error && (
          <p className="vault-setup-error" role="alert">
            {error}
          </p>
        )}
        <div className="inline-form">
          {recovery.restoreAvailable && (
            <button data-control="migration-recovery-restore"
              className="primary"
              type="button"
              disabled={busy}
              onClick={onRestore}
            >
              Restore verified backup
            </button>
          )}
          {recovery.retryAvailable && (
            <button data-control="migration-recovery-retry" type="button" disabled={busy} onClick={onRetry}>
              Retry migration
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
