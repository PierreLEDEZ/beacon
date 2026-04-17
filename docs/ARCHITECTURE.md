# Beacon — Architecture

> Équivalent Windows de Vibe Island, compatible Claude Code uniquement.

## 1. Contexte et objectifs

**But** : créer une app Windows qui affiche un "pseudo-notch" flottant en haut de l'écran et sert de tableau de bord pour les sessions Claude Code tournant dans WSL. Depuis ce notch, l'utilisateur :

- voit toutes ses sessions Claude Code actives (plusieurs en parallèle)
- approuve/refuse les demandes de permission sans quitter son éditeur
- répond aux questions (AskUserQuestion)
- relit les plans (ExitPlanMode) avec rendu Markdown
- saute vers le bon terminal (fenêtre + tab + pane) d'un clic
- reçoit des sons/notifications pour les events clés

## 2. Contraintes environnement

- **Windows 11** + **WSL2 Ubuntu** en mode mirrored (`127.0.0.1` partagé entre Windows et WSL)
- Claude Code tourne **dans WSL**, lancé depuis Ghostty ou VSCode
- L'utilisateur utilise souvent **Zellij** dans Ghostty (plusieurs panes = plusieurs sessions Claude en parallèle)
- Architecture **générique** : pas de hardcode Ghostty/Zellij, doit fonctionner aussi avec tmux, sans multiplexeur, Windows Terminal, WezTerm, etc.

## 3. Stack

- **Backend Tauri 2 (Rust)** sur Windows : serveur HTTP local (axum), état des sessions, APIs Win32, fenêtre notch, system tray
- **Frontend** (React + TypeScript + Vite + Tailwind) : rendu du notch, interactions
- **Hooks shell bash** côté WSL : scripts génériques déclenchés par Claude Code

## 4. Vue d'ensemble

```
┌─ WSL Ubuntu ──────────────────────┐       ┌─ Windows ─────────────────────┐
│                                   │       │                               │
│  Claude Code session A  ──┐       │       │  ┌─────────────────────────┐ │
│  Claude Code session B  ──┼──►    │       │  │  Beacon (Tauri app)     │ │
│  Claude Code session C  ──┘       │       │  │                         │ │
│       (n'importe où :             │ HTTP  │  │  ┌─ Rust backend ─────┐ │ │
│        ghostty/zellij,            │ local │  │  │ axum HTTP server   │ │ │
│        vscode term,               │ ────► │  │  │ SessionManager     │ │ │
│        windows term,              │◄──── │  │  │ EventBus (tokio)   │ │ │
│        etc.)                      │ (long-│  │  │ Win32 APIs         │ │ │
│                                   │ poll) │  │  └─────────┬──────────┘ │ │
│  ↕                                │       │  │            │ IPC        │ │
│  ~/.local/bin/beacon-hook         │       │  │  ┌─────────▼──────────┐ │ │
│    (script générique, détecte     │       │  │  │ Webview (React TS) │ │ │
│     contexte, POST à Beacon,      │       │  │  │ Pseudo-notch UI    │ │ │
│     long-poll décisions)          │       │  │  └────────────────────┘ │ │
│                                   │       │  └─────────────────────────┘ │
│  ~/.claude/settings.json          │       │                              │
│    (hooks → beacon-hook)          │       │  + system tray               │
│                                   │       │  + raccourcis globaux        │
└───────────────────────────────────┘       └──────────────────────────────┘
```

## 5. Flux principaux

### Flux 1 — Notification non bloquante

1. Claude Code déclenche un hook (ex. `PostToolUse`)
2. `beacon-hook` (bash) collecte le contexte d'exécution
3. `POST /event` avec le payload complet
4. Beacon met à jour l'état de la session et notifie le frontend via IPC
5. Le notch s'actualise
6. Le hook rend la main immédiatement (exit 0)

### Flux 2 — Permission bloquante

1. Claude Code déclenche `PreToolUse` (ex. avant un `Edit` ou `Bash`)
2. `beacon-hook` POST l'event avec `blocking: true` et `event_id`
3. Le hook **bloque** en faisant `GET /wait/:event_id` (long-poll, timeout configurable)
4. Beacon affiche la demande Allow/Deny dans le notch (avec diff pour les edits, commande pour les bash)
5. User clique Allow ou Deny (ou shortcut `Ctrl+Alt+Y` / `Ctrl+Alt+N`)
6. Frontend POST `/decision/:event_id`
7. Le long-poll côté hook se débloque avec la réponse
8. Le hook écrit le JSON de décision sur stdout
9. Claude Code agit selon la décision

### Flux 3 — Jump vers terminal

1. User clique sur une carte de session dans le notch
2. Frontend POST `/jump/:session_id`
3. Beacon consulte le contexte d'exécution stocké pour cette session
4. Pipeline de jump (génériquement) :
   - Étape 1 : si `host_terminal.kind` connu et HWND résolu → `SetForegroundWindow(hwnd)` (avec contournement focus-stealing Win32)
   - Étape 2 : si `multiplexer.kind` connu → `wsl.exe -d Ubuntu -- <cmd multiplexeur>`
     - zellij : `zellij --session <name> action focus-pane --pane-id <id>`
     - tmux : `tmux select-pane -t <target>`
     - screen : best-effort
5. Étapes manquantes = skippées silencieusement

## 6. Modèle de données

### Contexte d'exécution (envoyé par chaque event)

```json
{
  "event_id": "uuid-v4",
  "event_type": "PreToolUse" | "PostToolUse" | "UserPromptSubmit" | "Notification" | "Stop" | "SubagentStop" | "SessionStart" | "SessionEnd",
  "timestamp": "ISO-8601",
  "blocking": true | false,
  "timeout_ms": 30000,

  "claude": {
    "session_id": "...",
    "pid": 12345,
    "cwd": "/home/user/project",
    "transcript_path": "...",
    "tool_name": "Edit" | "Bash" | "Read" | "AskUserQuestion" | "ExitPlanMode" | ...,
    "tool_input": { /* payload du tool */ }
  },

  "execution_context": {
    "shell_pid": 12300,
    "tty": "/dev/pts/5",

    "multiplexer": {
      "kind": "zellij" | "tmux" | "screen" | null,
      "session": "...",
      "tab": "...",
      "pane": "..."
    },

    "host_terminal": {
      "kind": "ghostty" | "vscode-term" | "wezterm" | "windows-terminal" | "unknown",
      "markers": { /* env vars détectées, pour debug */ }
    }
  }
}
```

### Session (état interne, Rust)

```rust
struct Session {
    claude_session_id: String,
    first_seen: Instant,
    last_activity: Instant,
    status: Status,  // Idle, Working, WaitingApproval, Done, Error
    cwd: PathBuf,
    multiplexer: Option<MultiplexerLocation>,
    host_terminal: HostTerminal,
    current_hwnd: Option<isize>,
    pending_decision: Option<PendingDecision>,
    history: Vec<EventSummary>,
}
```

### Décision (réponse du user)

```json
{
  "decision": "allow" | "deny" | "answer",
  "reason": "...",
  "answer": "...",
  "modified_input": { }
}
```

## 7. Protocole HTTP (localhost:37421)

| Méthode | Route                   | Rôle                                      |
| ------- | ----------------------- | ----------------------------------------- |
| GET     | `/health`               | ping                                      |
| POST    | `/event`                | hook WSL soumet un event                  |
| GET     | `/wait/:event_id`       | hook WSL long-poll la décision (bloquant) |
| POST    | `/decision/:event_id`   | frontend enregistre la décision user      |
| GET     | `/sessions`             | liste sessions actives                    |
| GET     | `/sessions/:id/history` | historique d'une session                  |
| POST    | `/jump/:session_id`     | déclenche le jump                         |
| DELETE  | `/sessions/:id`         | oublie une session terminée               |

CORS restreint à `127.0.0.1` uniquement.

## 8. Hooks Claude Code supportés

| Hook               | Phase | Blocking | Action                                                                                  |
| ------------------ | ----- | -------- | --------------------------------------------------------------------------------------- |
| `SessionStart`     | 1     | non      | crée session                                                                            |
| `UserPromptSubmit` | 1     | non      | update session, affiche prompt court                                                    |
| `PreToolUse`       | 2     | **oui**  | Allow/Deny (+ Plan review si tool = ExitPlanMode, + Question si tool = AskUserQuestion) |
| `PostToolUse`      | 1     | non      | log tool exécuté, update statut                                                         |
| `Notification`     | 2     | non      | notification sonore, badge                                                              |
| `Stop`             | 1     | non      | session → Done                                                                          |
| `SubagentStop`     | 1     | non      | idem                                                                                    |
| `SessionEnd`       | 1     | non      | archive session                                                                         |

## 9. Détection générique (pas de hardcode)

### Multiplexeur (dans le hook shell)

Inspection des variables d'env dans cet ordre :

- `$ZELLIJ` non vide → zellij (+ `$ZELLIJ_SESSION_NAME`, `$ZELLIJ_PANE_ID`)
- `$TMUX` non vide → tmux (+ `$TMUX_PANE`, `tmux display-message -p` pour session/window)
- `$STY` non vide → screen
- sinon → null

### Terminal hôte (dans le hook shell)

Inspection des marqueurs env :

- `$TERM_PROGRAM=vscode` → vscode-term
- `$GHOSTTY_RESOURCES_DIR` non vide ou `$TERM=xterm-ghostty` → ghostty
- `$WEZTERM_EXECUTABLE` non vide → wezterm
- `$WT_SESSION` non vide → windows-terminal
- sinon → unknown

Fallback robuste : lecture de `/proc/$$/environ` du shell si les vars ne sont pas directement dans l'env du hook.

### HWND (côté Rust Windows)

1. Enumérer les HWND via `EnumWindows`
2. Pour chaque HWND, récupérer le `process_id`, puis le nom de l'exe (`QueryFullProcessImageNameW`)
3. Matcher selon `host_terminal.kind` :
   - ghostty → `ghostty.exe`
   - vscode-term → `Code.exe` ou `Code - Insiders.exe`
   - wezterm → `wezterm-gui.exe`
   - windows-terminal → `WindowsTerminal.exe`
4. Si plusieurs candidats : utiliser `GetForegroundWindow` au moment du premier event de la session, puis cacher

## 10. Structure du projet

```
beacon/
├── Cargo.toml                      # workspace
├── src-tauri/                      # backend Rust + Tauri
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── server/
│       │   ├── mod.rs              # axum setup
│       │   ├── routes.rs
│       │   └── dto.rs
│       ├── session/
│       │   ├── mod.rs              # SessionManager
│       │   ├── state.rs
│       │   └── context.rs
│       ├── events/
│       │   ├── mod.rs              # EventBus tokio broadcast
│       │   └── types.rs
│       ├── decisions/
│       │   ├── mod.rs              # pending decisions, timeouts
│       │   └── types.rs
│       ├── platform/               # Windows-specific
│       │   ├── window.rs           # config fenêtre notch
│       │   ├── hotkeys.rs
│       │   ├── tray.rs
│       │   ├── hwnd_finder.rs
│       │   └── wsl_bridge.rs       # appels wsl.exe
│       ├── jump/
│       │   ├── mod.rs              # pipeline de jump
│       │   ├── focus_window.rs
│       │   └── multiplexer.rs
│       ├── sounds/
│       │   └── mod.rs              # phase 3
│       ├── install/
│       │   └── hooks.rs            # install/update hooks Claude
│       └── config/
│           └── mod.rs
├── src/                            # frontend (Vite racine = src-tauri/../src)
│   ├── index.html
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── Notch.tsx
│   │   ├── SessionCard.tsx
│   │   ├── PermissionPrompt.tsx
│   │   ├── QuestionPrompt.tsx
│   │   └── PlanReview.tsx
│   └── lib/
│       └── tauri-client.ts
├── hooks/                          # scripts installés côté WSL
│   ├── beacon-hook
│   ├── beacon-hook.spec.md
│   └── install.sh
├── docs/
│   ├── ARCHITECTURE.md             # ce document
│   ├── PROTOCOL.md                 # détail HTTP Beacon ↔ hooks
│   └── PHASES.md
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

## 11. Phases de développement

### Phase 1 — Socle

Objectif : lancer `claude` dans WSL → voir la session apparaître dans le notch.

1. Bootstrap Tauri 2 + Vite + React + TS
2. Fenêtre notch Win32 (borderless, transparent, topmost, non-activating, centré top, arrondi)
3. États collapsed (200×32) / expanded (520×320) avec transition CSS
4. Serveur axum en background tokio task (routes `/health`, `/event`, `/sessions`)
5. SessionManager thread-safe (Arc<RwLock<HashMap>>)
6. EventBus tokio broadcast
7. IPC backend → frontend (emit à chaque update)
8. Notch affiche liste sessions + statut + dernière activité
9. Script `beacon-hook` bash générique (détection contexte multiplexer + host_terminal)
10. CLI `beacon --install-hooks` qui merge dans `~/.claude/settings.json`
11. Raccourci global `Ctrl+Alt+Space` show/hide
12. System tray avec menu Show/Hide/Quit

### Phase 2 — Interactions bloquantes

Objectif : gérer permissions, questions, plans depuis le notch.

1. `PreToolUse` blocking : `/wait/:id`, timeouts, état `WaitingApproval`
2. UI Permission Prompt (diff Edit, command Bash, boutons Allow/Deny + raison)
3. `AskUserQuestion` (détecté via tool_name) : question + choix / input free-text
4. `ExitPlanMode` (détecté via tool_name) : rendu Markdown + Approve/Feedback
5. Raccourcis contextuels `Ctrl+Alt+Y` / `Ctrl+Alt+N`
6. Politique timeout configurable (Deny/Allow/Ask après N minutes)
7. Queue visuelle si plusieurs demandes simultanées

### Phase 3 — Jump + sons + polish

Objectif : clic = saut précis, et vie quotidienne agréable.

1. HWND matching (EnumWindows + image name + cache par session)
2. `SetForegroundWindow` avec trick `AttachThreadInput` / `AllowSetForegroundWindow`
3. Jump multiplexeur via `wsl.exe -d Ubuntu --`
4. Sons 8-bit par event type, volume configurable
5. Persistance historique (rusqlite, `%APPDATA%\Beacon\history.db`)
6. Paramètres utilisateur (port, raccourcis, politique timeout, volume, autostart)
7. Icônes par tool type
8. Multi-écran (écran sous souris ou primaire, configurable)
9. Packaging `.msi` (Wix via Tauri bundler)

### Phase 4 — Extras

1. Corrélation session ↔ pane fine (fuser tty, socket zellij)
2. Sound packs custom
3. Stats (temps par session, tools les plus utilisés)
4. Mode Do Not Disturb
5. Webhooks (Discord/Slack) pour events critiques
6. Auto-update Tauri updater
7. Thèmes clair/sombre/custom

## 12. Décisions techniques par défaut

| Sujet               | Choix                                               |
| ------------------- | --------------------------------------------------- |
| Tauri               | **v2**                                              |
| Frontend            | **React + TypeScript + Vite**                       |
| Styling             | **Tailwind CSS**                                    |
| HTTP server         | **axum** (tokio)                                    |
| Persistance phase 3 | **rusqlite**                                        |
| Port HTTP           | **37421** (configurable)                            |
| Distro WSL          | **Ubuntu** (auto-détectée)                          |
| Raccourci show/hide | **Ctrl+Alt+Space**                                  |
| Timeout par défaut  | **5 min → Deny** (configurable)                     |
| Logging             | **tracing** + fichier dans `%APPDATA%\Beacon\logs\` |
| Hook shell          | **bash**                                            |

## 13. Risques identifiés

- **Focus stealing Windows** : `SetForegroundWindow` bridé. Contournement via `AttachThreadInput` + `AllowSetForegroundWindow`. Connu et gérable.
- **HWND ambigu** (plusieurs fenêtres Ghostty) : heuristique "foreground window au moment de l'event" + cache par session. Dégradation propre.
- **Relance `claude` dans un même pane** : nouveau `session_id`, même `pane_id`. Réassocier automatiquement.
- **NAT mode si user quitte mirrored** : fallback lecture `ip route show default` dans le hook.
- **Vars d'env manquantes dans hook non-interactif** : lecture `/proc/$$/environ` du shell.
- **Beacon pas lancé quand hook se déclenche** : timeout court de connexion (500ms), fallback "Allow" ou "Deny" selon politique.
