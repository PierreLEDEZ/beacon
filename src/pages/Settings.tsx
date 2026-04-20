import { useEffect, useState } from "react";

import {
  getSettings,
  updateSettings,
  type Settings as SettingsShape,
} from "../lib/tauri-client";

type Status = "idle" | "loading" | "saving" | "saved" | "error";

export function Settings() {
  const [settings, setSettings] = useState<SettingsShape | null>(null);
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then((s) => {
        setSettings(s);
        setStatus("idle");
      })
      .catch((e) => {
        setError(String(e));
        setStatus("error");
      });
  }, []);

  async function save() {
    if (!settings) return;
    setStatus("saving");
    setError(null);
    try {
      const saved = await updateSettings(settings);
      setSettings(saved);
      setStatus("saved");
      window.setTimeout(() => setStatus("idle"), 1600);
    } catch (e) {
      setError(String(e));
      setStatus("error");
    }
  }

  if (!settings) {
    return (
      <div className="settings-page">
        <div className="settings-status">
          {status === "error" ? error : "Loading…"}
        </div>
      </div>
    );
  }

  return (
    <div className="settings-page">
      <h1>Beacon settings</h1>
      <p className="settings-hint">
        Changes apply immediately for the decision timeout and WSL distro.
        The HTTP port change takes effect on the next Beacon launch.
      </p>

      <label className="settings-row">
        <span>HTTP port</span>
        <input
          type="number"
          min={1024}
          max={65535}
          value={settings.port}
          onChange={(e) =>
            setSettings({ ...settings, port: Number(e.target.value) })
          }
        />
      </label>

      <label className="settings-row">
        <span>WSL distro</span>
        <input
          type="text"
          value={settings.wsl_distro}
          onChange={(e) =>
            setSettings({ ...settings, wsl_distro: e.target.value })
          }
          placeholder="Ubuntu"
        />
      </label>

      <label className="settings-row">
        <span>Decision timeout (seconds)</span>
        <input
          type="number"
          min={5}
          max={3600}
          value={settings.decision_timeout_secs}
          onChange={(e) =>
            setSettings({
              ...settings,
              decision_timeout_secs: Number(e.target.value),
            })
          }
        />
      </label>

      <label className="settings-row">
        <span>Notch monitor</span>
        <select
          value={settings.notch_monitor}
          onChange={(e) =>
            setSettings({
              ...settings,
              notch_monitor: e.target.value as "cursor" | "primary",
            })
          }
        >
          <option value="cursor">Under the cursor (at launch)</option>
          <option value="primary">Always primary</option>
        </select>
      </label>

      {error && <div className="settings-error">{error}</div>}

      <div className="settings-actions">
        <span className="settings-status">
          {status === "saving" && "Saving…"}
          {status === "saved" && "Saved"}
        </span>
        <button
          className="btn-allow"
          disabled={status === "saving"}
          onClick={save}
        >
          Save
        </button>
      </div>
    </div>
  );
}
