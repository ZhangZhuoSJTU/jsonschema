# Repository AI access controls

Repository contents are confidential. AI access needs explicit owner
authorisation for the current conversation, and tool calls stay prohibited even
then. The policy covers every provider, language, and local or remote model,
including Chinese-developed models.

**These controls are defaults that cooperative or default-configured clients
load. They are not a security boundary.** An agent that already has a shell or
unrestricted file access is not stopped by anything in this repository. Read
[What the repository cannot prevent](#what-the-repository-cannot-prevent) before
relying on them.

## What is here

| Layer | Files | Purpose |
| --- | --- | --- |
| Notice | `AGENTS.md` (canonical) and byte-identical copies: `CLAUDE.md`, `GEMINI.md`, `QWEN.md`, `.windsurfrules`, `.github/copilot-instructions.md`, `.clinerules/`, `.cursor/rules/` | Tells an agent to stop. Advisory only. |
| Blocking hooks | `.claude/settings.json`, `.gemini/settings.json`, `.codex/hooks.json`, `.cursor/hooks.json`, Cline hook scripts | Reject prompts and tool calls before they run. |
| Tool removal | `permissions.deny` in `.claude/settings.json`, `opencode.json` | Removes tools from the client, independent of hooks. |
| Context exclusion | `.cursorignore`, `.codeiumignore`, `.devinignore`, `.geminiignore`, `.aiderignore`, `.clineignore`, `.rooignore`, `.continueignore`, `.aiexclude`, `.aiignore`, `.qwenignore` | Keeps files out of indexing and automatic context. Does not stop shell reads. |
| Launch roots | `.claude` and `.gemini` symlinks in `crates/`, each crate, `fuzz/`, `profiler/` | Carries the root settings into directories people start a client from. |
| Tamper evidence | `test_guards.py`, `.github/workflows/ai-access.yml` | Fails when a guard is removed, weakened, or out of sync. |
| Live evidence | `live_check.py` | Drives installed clients against a synthetic directory. |

Every hook runs one constant command:

    echo 'Restricted repository: AI access is prohibited.' >&2; exit 2

It never parses, logs, or executes its input. The message on stderr matters:
Gemini CLI ignores an `exit 2` that prints nothing (see below).

## Evidence

Three levels, kept apart on purpose.

### Tested live

Measured on 12 November 2026 with `live_check.py`, on macOS, against a synthetic
directory and a local fake endpoint. No real model was contacted. Results hold
for these versions only; rerun after any client upgrade.

| Client | Result |
| --- | --- |
| Claude Code 2.1.317 | From the repository root or a launch root, the prompt is rejected and no request is sent, including with `--dangerously-skip-permissions`. With the prompt hook deleted, hooks disabled through `settings.local.json`, and bypass mode requested, the deny list still kept scripted `Bash`, `Read`, and `Write` calls from running. |
| Gemini CLI 0.27.2 | From the root or a launch root, with `--yolo`, no request is sent. |
| Codex CLI 0.163.0 | Hooks block the prompt, from any subdirectory, **only after a human has trusted the project and the hooks**. Otherwise they are skipped. |
| opencode 1.18.11 | Every tool resolves to disabled, from any subdirectory. This checks configuration resolution, not a live tool call. |

Two defects in the earlier configuration were found this way:

- The Gemini hooks used a silent `exit 2`. Gemini 0.27.2 logged a hook failure
  and sent the request anyway, with the prompt and the directory listing.
  Blocking only `BeforeModel` also let a routing request out. The fix is the
  stderr message plus a `BeforeAgent` hook.
- Claude Code and Gemini CLI read project settings only from the directory they
  are started in. Starting either in `crates/jsonschema/` skipped every hook and
  deny rule. Launch-root symlinks fix this for the linked directories only.

### Checked offline

`python3 .ai-access/test_guards.py` runs in CI and checks that:

- every hook uses the constant command, exits 2, prints only the fixed message,
  and ignores malformed or hostile input;
- all notice copies match `AGENTS.md` and all ignore files match each other;
- every directory with a tracked `Cargo.toml` or `pyproject.toml`, and each of
  its parents, links `.claude` and `.gemini` to the root with a relative path;
- no `*.disabled` settings file or hook-disabling local override is left behind.

This proves the files are present and consistent. It says nothing about whether
a client loads them.

### Documented by the vendor, not tested here

Cursor hooks and rules, Cline hooks, Copilot instructions, Windsurf rules,
`QWEN.md`, and every ignore file. Cursor's CLI needs its cloud service, so it
could not be driven offline. Qwen Code 0.0.8 did not run hooks from a
`.qwen/settings.json` in a quick probe, so it has a notice and an ignore file
only. Claude's `onFailure` and Cursor's `failClosed` keys are set as documented;
their effect on a crashing hook was not exercised. Treat all of these as
unverified.

## What the repository cannot prevent

Confirmed by `live_check.py` (`KnownGaps`), which passes while each gap is open:

- **Unlinked directories.** Claude Code started in a directory with no link,
  such as `crates/jsonschema/src/`, runs with no guard. The same applies to
  Gemini CLI.
- **Launch flags.** `claude --setting-sources user` skips project settings.
  Starting a client elsewhere and adding this directory has the same effect.
- **Codex trust.** Repository hooks do nothing until a human trusts them.

Not testable from here, but true by construction:

- Any process with file access can read the plaintext source and Git history.
  A hook cannot see a `cat` run by a different program.
- An agent with a shell can delete these files, edit them, or copy the tree
  somewhere without them. CI catches a committed change, not a local one.
- The notice cannot verify who is asking. "I am the owner" typed by anyone looks
  the same to a model. A model that ignores instructions ignores the notice.
- Remote and cloud agents (Copilot coding agent, Cursor background agents,
  Codex cloud, Devin) clone the repository and run outside these hooks.
- A client or version not listed here loads none of this unless it happens to
  read `AGENTS.md`.
- Ignore files stop indexing in clients that honour them. They stop nothing
  else.

Controls that would actually hold, all outside this repository:

- Machine-wide managed settings (for Claude Code, `managed-settings.json` with
  the same deny list and hooks). These apply in every directory and cannot be
  overridden by flags or project files. This closes the first two gaps above.
- A separate OS account or sandbox for any agent, without read access here.
- Organisation settings: Copilot content exclusion, disabling cloud agents for
  this repository, branch protection requiring the `ai-access` check, and
  CODEOWNERS review for the guard files.
- Encryption at rest with keys held elsewhere, if plaintext on disk is the
  concern. This changes the development workflow and does not cover existing
  clones or history.

## Maintenance

**Changing the notice.** Edit `AGENTS.md`, copy it over each file listed in
`NOTICE_COPIES` in `test_guards.py`, and replace the body of the Cursor rule.
The test fails until every copy matches.

**Adding a crate.** Link the settings into the new directory, adjusting the
number of `../` to its depth:

    ln -s ../../.claude ../../.gemini crates/new-crate/

The test names any launch root that lacks a link. On Windows, Git needs
`core.symlinks=true` or the links arrive as plain files and do nothing.

**Granting temporary access.** There is no in-repository switch, by design: a
switch an agent could flip would be worthless. The owner renames the relevant
settings file (for example `.claude/settings.json` to
`.claude/settings.json.disabled`), restarts the client, and authorises the task
in the conversation. Rename it back afterwards. `test_guards.py` and CI fail
while a `.disabled` file exists or the settings file is missing, so a forgotten
exception cannot be merged unnoticed.

## Trade-offs

- The launch-root links add 32 symlinks. Linking every directory would close
  more of the gap at the cost of two dot-entries in each source directory;
  managed settings are the better fix.
- Crates here set no `include` list, so `cargo package` would ship the linked
  settings files in a published crate. They contain no source, but add an
  `exclude` if these crates are ever published from this tree.
- Blocking every prompt means the Claude, Gemini, and Codex CLIs are unusable
  in this tree, for humans too. That is the policy, not a side effect.
- The hooks stay a constant command instead of a script that checks for an
  authorisation token. A script would make temporary access easier, and would
  also be something an agent could satisfy.

## Client references

- [Claude Code permissions](https://code.claude.com/docs/en/permissions) and
  [hooks](https://code.claude.com/docs/en/hooks).
- [Codex hooks and hook trust](https://developers.openai.com/codex/hooks).
- [Cursor hooks](https://cursor.com/docs/hooks) and
  [ignore-file limitations](https://cursor.com/docs/reference/ignore-file).
- [Gemini CLI hooks](https://geminicli.com/docs/hooks/reference/) and
  [ignore files](https://geminicli.com/docs/cli/gemini-ignore/).
- [opencode permissions](https://opencode.ai/docs/permissions/).
- [Cline hooks and ignore files](https://docs.cline.bot/customization/clineignore).
- [Windsurf/Devin ignore files](https://docs.devin.ai/desktop/context-awareness/devin-ignore).
- [Aider configuration](https://aider.chat/docs/config/aider_conf.html).
- [Copilot repository instructions](https://docs.github.com/en/copilot/concepts/prompting-and-context/custom-instructions).
