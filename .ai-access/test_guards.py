"""Offline checks for repository AI restrictions; never starts an AI client.

These tests prove that the guard *configuration* is present, consistent and
self-blocking. They cannot prove that an installed client loads it: that is the
job of the opt-in `live_check.py`, which drives real client binaries against a
synthetic directory and a local fake endpoint.
"""

import json
from pathlib import Path, PurePosixPath
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent

# One constant command for every hook. The stderr text is load-bearing: Gemini
# CLI 0.27.2 treats `exit 2` with empty stderr as a hook failure and carries on,
# so a silent `exit 2` blocks nothing there (found by live_check.py).
DENY_MESSAGE = "Restricted repository: AI access is prohibited."
DENY_COMMAND = f"echo '{DENY_MESSAGE}' >&2; exit 2"

CLIENT_HOOKS = {
    ".claude/settings.json": ("PreToolUse", "UserPromptSubmit"),
    ".cursor/hooks.json": (
        "beforeSubmitPrompt", "preToolUse", "beforeReadFile", "beforeTabFileRead",
        "beforeShellExecution", "beforeMCPExecution", "subagentStart",
    ),
    # BeforeAgent is the event that stops a Gemini turn before anything is sent;
    # BeforeModel alone still lets the routing classifier request out.
    ".gemini/settings.json": (
        "BeforeAgent", "BeforeModel", "BeforeToolSelection", "BeforeTool",
    ),
    ".codex/hooks.json": ("UserPromptSubmit", "PreToolUse", "PermissionRequest"),
}
CLINE_HOOKS = (".clinerules/hooks/PreToolUse", ".cline/hooks/PreToolUse.sh")

# AGENTS.md is the canonical notice; every other copy must match it byte for
# byte so that a policy edit cannot silently reach only some clients.
NOTICE = "AGENTS.md"
NOTICE_COPIES = (
    "CLAUDE.md", "GEMINI.md", "QWEN.md", ".windsurfrules",
    ".github/copilot-instructions.md", ".clinerules/00-restricted-repository.md",
)
CURSOR_RULE = ".cursor/rules/00-restricted-repository.mdc"

IGNORE_FILES = (
    ".cursorignore", ".codeiumignore", ".devinignore", ".geminiignore",
    ".aiderignore", ".clineignore", ".rooignore", ".continueignore",
    ".aiexclude", ".aiignore", ".qwenignore",
)

# Claude Code and Gemini CLI read project settings only from the directory they
# are launched in (verified live), so each directory a developer is likely to
# launch from carries a symlink back to the root settings directory.
LINKED_SETTINGS = (".claude", ".gemini")
MANIFESTS = ("Cargo.toml", "pyproject.toml")


def read_config(name):
    return json.loads((ROOT / name).read_text())


def handlers_of(name, group):
    """Cursor puts the command on the group itself; the others nest a list."""
    return [group] if name.startswith(".cursor/") else group["hooks"]


def launch_roots():
    """Directories holding a tracked package manifest, plus their ancestors."""
    tracked = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.split("\0")
    roots = set()
    for name in tracked:
        path = PurePosixPath(name)
        if path.name in MANIFESTS:
            roots.update(str(parent) for parent in path.parents if str(parent) != ".")
    return sorted(roots)


class ToolGuardsTest(unittest.TestCase):
    def test_claude_denies_alternative_access_routes(self):
        permissions = read_config(".claude/settings.json")["permissions"]
        for tool in ("Read", "Write", "Edit", "Bash", "PowerShell", "Glob",
                     "Grep", "WebFetch", "WebSearch", "Agent", "Task", "mcp__*"):
            self.assertIn(tool, permissions["deny"])
        self.assertEqual(permissions["disableBypassPermissionsMode"], "disable")

    def test_hooks_cover_every_tool_and_fail_closed_where_supported(self):
        for name, events in CLIENT_HOOKS.items():
            for event in events:
                with self.subTest(client=name, event=event):
                    groups = read_config(name)["hooks"][event]
                    self.assertTrue(groups)
                    for group in groups:
                        self.assertIn(group.get("matcher"), (None, "", "*", ".*"))
                        for handler in handlers_of(name, group):
                            self.assertEqual(handler["command"], DENY_COMMAND)
                            if name.startswith(".claude/"):
                                self.assertEqual(handler["onFailure"], "block")
                            elif name.startswith(".cursor/"):
                                self.assertIs(handler["failClosed"], True)

    def test_configured_commands_reject_untrusted_payloads_without_disclosure(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "must-not-exist"
            secret = "SYNTHETIC-CONFIDENTIAL-MARKER"
            payloads = ["", "not-json", json.dumps({
                "tool_name": "unrecognised_future_tool",
                "model": "arbitrary-local-model",
                "tool_input": {"command": f"touch {marker}", "content": secret},
            })]
            for name, events in CLIENT_HOOKS.items():
                for event in events:
                    for group in read_config(name)["hooks"][event]:
                        for handler in handlers_of(name, group):
                            for payload in payloads:
                                with self.subTest(client=name, event=event, payload=payload):
                                    result = subprocess.run(
                                        ["/bin/sh", "-c", handler["command"]], input=payload,
                                        text=True, capture_output=True, cwd=directory, timeout=5,
                                    )
                                    self.assertEqual(result.returncode, 2)
                                    self.assertEqual(result.stdout, "")
                                    # Constant, non-empty reason: never derived from input.
                                    self.assertEqual(result.stderr.strip(), DENY_MESSAGE)
                                    self.assertFalse(marker.exists())

    def test_cline_hooks_cancel_without_echoing_input(self):
        for name in CLINE_HOOKS:
            with self.subTest(hook=name):
                path = ROOT / name
                self.assertTrue(path.stat().st_mode & 0o111)
                for payload in ("", "invalid", '{"tool_name":"future_tool","secret":"private"}'):
                    result = subprocess.run(
                        ["/bin/sh", str(path)], input=payload, text=True,
                        capture_output=True, cwd="/tmp", timeout=5,
                    )
                    self.assertEqual(result.returncode, 0)
                    self.assertIs(json.loads(result.stdout)["cancel"], True)
                    self.assertNotIn("private", result.stdout + result.stderr)

    def test_opencode_disables_every_tool_and_sharing(self):
        config = read_config("opencode.json")
        self.assertEqual(config["permission"], {"*": "deny"})
        self.assertEqual(config["share"], "disabled")

    def test_ignore_files_exclude_source_hidden_files_and_nested_content(self):
        paths = ("source.rs", "crates/example/src/lib.rs", ".env", ".git/config",
                 ".hidden/nested/secret", "docs/private.md", "new-folder/new-file")
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(["git", "init", "--quiet", directory], check=True,
                           capture_output=True)
            for name in IGNORE_FILES:
                with self.subTest(ignore=name):
                    (Path(directory) / ".gitignore").write_text((ROOT / name).read_text())
                    result = subprocess.run(
                        ["git", "-c", "core.excludesFile=/dev/null", "check-ignore",
                         "--no-index", "--stdin"], input="\n".join(paths) + "\n",
                        text=True, capture_output=True, cwd=directory, check=True,
                    )
                    self.assertEqual(set(result.stdout.splitlines()), set(paths))


class IntegrityTest(unittest.TestCase):
    """Catches drift, weakening and forgotten temporary exceptions."""

    def test_guard_directories_do_not_break_workspace_discovery(self):
        result = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--offline", "--format-version", "1"],
            cwd=ROOT, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        packages = json.loads(result.stdout)["packages"]
        self.assertTrue(packages)
        self.assertFalse(any(
            Path(package["manifest_path"]).parent.name in LINKED_SETTINGS
            for package in packages
        ))

    def test_every_notice_copy_matches_the_canonical_notice(self):
        notice = (ROOT / NOTICE).read_bytes()
        for name in NOTICE_COPIES:
            with self.subTest(copy=name):
                self.assertEqual((ROOT / name).read_bytes(), notice)
        # The Cursor rule is the same notice behind its own front matter.
        self.assertTrue((ROOT / CURSOR_RULE).read_bytes().endswith(notice))
        self.assertIn(b"alwaysApply: true", (ROOT / CURSOR_RULE).read_bytes())

    def test_every_ignore_file_is_identical(self):
        reference = (ROOT / IGNORE_FILES[0]).read_bytes()
        for name in IGNORE_FILES[1:]:
            with self.subTest(ignore=name):
                self.assertEqual((ROOT / name).read_bytes(), reference)

    def test_no_temporary_exception_is_left_behind(self):
        # Granting access means renaming a settings file out of the way. A
        # leftover rename, or a local override that switches hooks off, would
        # turn a one-conversation exception into a permanent one.
        leftovers = [
            str(path.relative_to(ROOT))
            for directory in (".claude", ".gemini", ".cursor", ".codex")
            for path in (ROOT / directory).iterdir()
            if path.name.endswith((".disabled", ".bak", ".off"))
        ]
        self.assertEqual(leftovers, [])
        local = ROOT / ".claude" / "settings.local.json"
        if local.exists():
            self.assertFalse(json.loads(local.read_text()).get("disableAllHooks", False))

    def test_launch_roots_link_to_the_root_settings(self):
        roots = launch_roots()
        self.assertIn("crates/jsonschema", roots)  # the discovery itself works
        for directory in roots:
            for settings in LINKED_SETTINGS:
                with self.subTest(directory=directory, settings=settings):
                    link = ROOT / directory / settings
                    self.assertTrue(link.is_symlink(), f"missing: see README, 'Launch roots'")
                    # Relative, so the link survives a clone to another path
                    # and can never point outside the repository.
                    self.assertFalse(Path(link.readlink()).is_absolute())
                    self.assertEqual(link.resolve(), (ROOT / settings).resolve())

    def test_ci_runs_these_checks(self):
        workflow = (ROOT / ".github/workflows/ai-access.yml").read_text()
        self.assertIn("python3 .ai-access/test_guards.py", workflow)


if __name__ == "__main__":
    unittest.main()
