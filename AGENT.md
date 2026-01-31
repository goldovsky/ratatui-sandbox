CALLBOT rataTUI agent spec

Purpose
- Describe requirements and design for a small terminal UI (TUI) used by the CALLBOT team.
- The TUI is a thin interface that helps users discover and run existing scripts. In development it echoes constructed commands instead of performing side effects.

Scope
- Two categories of scripts:
  - Project-scoped (operate on current git project / branch): `cmr`, `fmr`, `deploySnapshot`, etc.
  - Server-scoped (require `{ENV}` and `{SERVER}` arguments): examples of ENV = `qlf`, `pprod`, `prod`; SERVER = `och01`, `och02`, `eva01`, `mco01`.
- The TUI builds the command and either echoes it (dev/dry-run) or spawns a child process and hands the TTY to that process (run mode).

User stories
- Browse available actions without memorising exact script names and args.
- Select or type `ENV` and `SERVER` (with suggestions) for server scripts.
- Preview the exact shell command before it runs and cancel if needed.
- Run an action and receive control of the terminal for interactive scripts.

High-level UI
- Main menu: two tiles/buttons: `Project Actions` and `Server Actions`.
- Project Actions list: shows project scripts with short descriptions. Selecting one opens an argument form if needed.
- Server Actions list: opens a form that collects `ENV` and `SERVER` with previews.
- Preview screen: show the exact command, and offer `Echo` (dry-run) and `Run`.

Execution model
- Build a single command string (shell-escaped as needed).
- Dev/dry-run: echo the command and return (no side effects).
- Run mode: spawn a shell and hand the TTY to the child process so interactive scripts work (use standard OS facilities to inherit stdio).

Design / implementation notes
- Tech: Rust + `ratatui` (terminal UI) with `crossterm` backend.
- Suggested source layout:
  - `src/ui/*` - screens and components (MainMenu, ListView, FormView, Preview)
  - `src/runner.rs` - command builder and runner (spawns shell via std::process::Command)
  - `config/environments.json` - list of ENV + servers and friendly names
  - `scripts/echo-*` - helper scripts for local dev

- Data model (simple spec):

```text
ActionSpec {
  id: String,
  kind: project | server,
  label: String,
  description: Option<String>,
  args: Vec<String>,
  template: String // e.g. "./scripts/cmr.sh --branch {branch}"
}
```

UX details and validations
- ENV and SERVER fields: present as picklists with typeahead; allow custom values but warn when unknown.
- Remember last used values in a small JSON file in the user's config dir (use the `directories` crate).
- Provide a `Dry run` toggle that forces echo behavior.

Development helper scripts
- Add `scripts/echo-cmr.sh`, `scripts/echo-fmr.sh`, `scripts/echo-deploySnapshot.sh` that simply echo given args for fast iteration.

Example quick workflow (developer)
1. Start the TUI.
2. Navigate: Main menu → Project Actions → `deploySnapshot` → enter branch `feature/xyz` → Preview → Echo (dev mode) or Run.

Next steps
1. Create `src` scaffold and a minimal `ratatui` app showing the main menu.
2. Add the `ActionSpec` list and implement Preview + Runner wiring.
3. Add `scripts/echo-*` files and a `config/environments.json` for server picklists.

Design update (final UI spec)
- App runs fullscreen and clears the terminal on start; it uses the full terminal space.
- Header: centered title and a short description.
- Main area: two columns — each column has a title and a scrollable list of scripts. Columns show focus and support keyboard navigation (Tab to switch columns, arrows/vi keys inside list).
- Bottom area: full-width bordered preview box showing the current command preview or contextual help.
- Detail screens: action-specific screens replace the two-column listing and provide argument forms, preview, and controls.

Implementation note
- Start without handing over a TTY (dry-run mode) to iterate quickly; add the run-mode TTY handover as a separate, well-tested step.

Last updated: 2026-01-31
