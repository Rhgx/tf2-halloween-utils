import { LogIn, TriangleAlert } from 'lucide-react';

import type { GameState, ServerState, Settings, SettingsPatch } from '../../api.ts';
import { Field, Group, Switch } from '../../components/fields.tsx';

type SettingsViewProps = {
  className: string;
  settings: Settings | null;
  server: ServerState | null;
  game: GameState | null;
  syncedAt: string | null;
  onChange: (patch: SettingsPatch) => void;
  onChooseFolder: () => void;
  onUseDetected: () => void;
  onOpenData: () => void;
  onSignIn: () => void;
  onSignOut: () => void;
  onRetry: () => void;
};

const PLATFORMS: Record<Settings['platform'], string | null> = { windows: 'Windows', linux: 'Linux', other: null };

// Things you set once. Changes apply as you make them; App rolls a failed one back.
export function SettingsView({
  className,
  settings,
  server,
  game,
  syncedAt,
  onChange,
  onChooseFolder,
  onUseDetected,
  onOpenData,
  onSignIn,
  onSignOut,
  onRetry
}: SettingsViewProps) {
  const platform = settings && PLATFORMS[settings.platform];
  const tf2Folder = settings?.tf2Folder ?? settings?.tf2FolderDetected;

  return (
    <section id="view-settings" className={className} role="tabpanel" aria-labelledby="tab-settings">
      <main className="wrap settings">
        {!settings ? (
          <p className="empty">
            Couldn't load the settings.{' '}
            <button type="button" className="link" onClick={onRetry}>
              Try again
            </button>
          </p>
        ) : (
          <>
            <Group title="App">
              <Field name="Close to tray">
                <Switch
                  label="Close to tray"
                  checked={settings.closeToTray}
                  onChange={(closeToTray) => onChange({ closeToTray })}
                />
              </Field>
              <Field name="Start minimized to tray">
                <Switch
                  label="Start minimized to tray"
                  checked={settings.startMinimized}
                  onChange={(startMinimized) => onChange({ startMinimized })}
                />
              </Field>
              {settings.platform === 'windows' && (
                <Field name="Start with Windows">
                  <Switch
                    label="Start with Windows"
                    checked={settings.startWithWindows}
                    onChange={(startWithWindows) => onChange({ startWithWindows })}
                  />
                </Field>
              )}
            </Group>

            <Group title="Game">
              <Field
                wide
                name="TF2 folder"
                help={!tf2Folder && 'Not found in your Steam libraries. Choose the folder that holds the tf folder.'}
              >
                <span className="path">
                  <code>{tf2Folder ?? 'Not found'}</code>
                  <button type="button" className="btn small" onClick={onChooseFolder}>
                    Change
                  </button>
                  <button
                    type="button"
                    className="btn small"
                    disabled={!settings.tf2FolderOverridden || !settings.tf2FolderDetected}
                    onClick={onUseDetected}
                  >
                    Use detected
                  </button>
                </span>
              </Field>
              <Field
                name="Match tracking"
                help={
                  game &&
                  !game.logFound && (
                    <>
                      Add <code>-condebug</code> to TF2's launch options in Steam.
                    </>
                  )
                }
              >
                {!game ? (
                  <span className="state">Unknown</span>
                ) : game.logFound ? (
                  <span className="state ok">console.log found</span>
                ) : (
                  <span className="state warn">
                    <TriangleAlert aria-hidden="true" />
                    console.log not found
                  </span>
                )}
              </Field>
            </Group>

            <Group title="Steam">
              <Field name="Account" help={server?.loggedIn && (syncedAt ? `Last sync ${syncedAt}` : 'Not synced yet')}>
                {server?.loggedIn ? (
                  <button type="button" className="btn small" onClick={onSignOut}>
                    Sign out
                  </button>
                ) : (
                  <button type="button" className="btn small" disabled={!server} onClick={onSignIn}>
                    <LogIn aria-hidden="true" />
                    Sign in with Steam
                  </button>
                )}
              </Field>
            </Group>

            <Group title="Contracts">
              <Field name="Restore original casual map selection on quit">
                <Switch
                  label="Restore original casual map selection on quit"
                  checked={settings.restoreQueueOnQuit}
                  onChange={(restoreQueueOnQuit) => onChange({ restoreQueueOnQuit })}
                />
              </Field>
            </Group>

            <Group title="Data">
              <Field wide name="Data folder" help="Holds your Steam sign-in. Don't share it.">
                <span className="path">
                  <code>{settings.dataFolder}</code>
                  <button type="button" className="btn small" onClick={onOpenData}>
                    Open folder
                  </button>
                </span>
              </Field>
            </Group>

            <Group title="About">
              <Field name={`TF2 Halloween Utils ${settings.version}${platform ? ` on ${platform}` : ''}`}>
                <a
                  className="link muted"
                  href="https://github.com/Rhgx/tf2-halloween-utils"
                  target="_blank"
                  rel="noreferrer"
                >
                  GitHub
                </a>
              </Field>
            </Group>
          </>
        )}
      </main>
    </section>
  );
}
