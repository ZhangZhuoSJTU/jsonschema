"""Opt-in live checks: do installed AI clients actually honour the guards?

    AI_ACCESS_LIVE=1 python3 .ai-access/live_check.py

`test_guards.py` shows the configuration is well formed. This file shows what
real client binaries do with it, which is the only evidence that counts.

Safety properties of the harness itself:

* No real model is contacted. Each client is pointed at a server this file
  starts on 127.0.0.1. That server either answers HTTP 500 (a "sink" that only
  records what the client tried to send) or plays a scripted fake model.
* No repository source is exposed. Clients run in a throwaway directory that
  holds synthetic files plus copies of the guard configuration.
* Clients get an empty HOME, so the developer's own credentials, settings and
  trust decisions neither leak into nor distort the result.

A client that is not installed is skipped, not failed. Results are tied to the
client versions you have installed: rerun after every client upgrade.
"""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import unittest


ROOT = Path(__file__).resolve().parent.parent
SECRET = "SYNTHETIC-SECRET-0001"
PROMPT = "synthetic probe prompt"
# Guard configuration copied into the synthetic repository.
GUARD_PATHS = ("AGENTS.md", "CLAUDE.md", "GEMINI.md", ".claude", ".gemini",
               ".codex", "opencode.json")
LINKED_SETTINGS = (".claude", ".gemini")


class FakeEndpoint:
    """Local stand-in for a model API that records every request body.

    With `script=None` it is a sink: every request gets HTTP 500, and the test
    asserts on how many requests arrived (ideally zero: blocked before send).

    With a script (a list of `{"name", "input"}` tool calls) it imitates the
    Anthropic Messages API: the first answer asks the client to run those
    tools, the second ends the turn. This is how the tool-level guards are
    exercised without a model deciding anything.
    """

    def __init__(self, script=None):
        self.requests = []
        endpoint = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                # Recorded too: a GET (for example a websocket upgrade attempt)
                # is still a client trying to reach its model.
                endpoint.requests.append((self.path, ""))
                self.reply(404, {})

            def do_POST(self):
                length = int(self.headers.get("content-length", 0))
                raw = self.rfile.read(length).decode("utf-8", "replace")
                endpoint.requests.append((self.path, raw))
                if script is None or "/v1/messages" not in self.path:
                    return self.reply(500, {})
                if "count_tokens" in self.path:
                    return self.reply(200, {"input_tokens": 1})
                body = json.loads(raw)
                offered = {tool.get("name") for tool in body.get("tools", [])}
                calls = [
                    {"type": "tool_use", "id": f"toolu_{index:02d}", **call}
                    for index, call in enumerate(script) if call["name"] in offered
                ]
                # Once a tool result is present the "model" has had its turn.
                if not calls or '"tool_result"' in raw:
                    content, stop = [{"type": "text", "text": "done"}], "end_turn"
                else:
                    content, stop = calls, "tool_use"
                message = {
                    "id": "msg_fake", "type": "message", "role": "assistant",
                    "model": body.get("model", "fake"), "content": content,
                    "stop_reason": stop, "stop_sequence": None,
                    "usage": {"input_tokens": 1, "output_tokens": 1},
                }
                if body.get("stream"):
                    return self.stream(message)
                self.reply(200, message)

            def reply(self, status, payload):
                data = json.dumps(payload).encode()
                self.send_response(status)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def stream(self, message):
                """Minimal server-sent-event rendering of one Messages reply."""
                self.send_response(200)
                self.send_header("content-type", "text/event-stream")
                self.end_headers()

                def event(kind, payload):
                    data = json.dumps(dict(payload, type=kind))
                    self.wfile.write(f"event: {kind}\ndata: {data}\n\n".encode())
                    self.wfile.flush()

                event("message_start",
                      {"message": dict(message, content=[], stop_reason=None)})
                for index, block in enumerate(message["content"]):
                    if block["type"] == "text":
                        start = {"type": "text", "text": ""}
                        delta = {"type": "text_delta", "text": block["text"]}
                    else:
                        start = dict(block, input={})
                        delta = {"type": "input_json_delta",
                                 "partial_json": json.dumps(block["input"])}
                    event("content_block_start", {"index": index, "content_block": start})
                    event("content_block_delta", {"index": index, "delta": delta})
                    event("content_block_stop", {"index": index})
                event("message_delta", {
                    "delta": {"stop_reason": message["stop_reason"], "stop_sequence": None},
                    "usage": {"output_tokens": 1},
                })
                event("message_stop", {})

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self.server.server_address[1]}"
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()

    def sent(self):
        """Everything any client transmitted, as one searchable string."""
        return "\n".join(body for _, body in self.requests)


class LiveCase(unittest.TestCase):
    client = None  # binary name; subclasses set it

    def setUp(self):
        if os.environ.get("AI_ACCESS_LIVE") != "1":
            self.skipTest("set AI_ACCESS_LIVE=1 to drive installed AI clients")
        if not shutil.which(self.client):
            self.skipTest(f"{self.client} is not installed")
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        base = Path(self.temporary.name).resolve()
        self.home = base / "home"
        self.home.mkdir()
        self.repo = base / "repo"
        self.build_synthetic_repository()

    def build_synthetic_repository(self):
        """A stand-in repository: synthetic content, the real guard files."""
        self.repo.mkdir()
        subprocess.run(["git", "init", "--quiet", str(self.repo)], check=True)
        (self.repo / "secret.txt").write_text(SECRET + "\n")
        for name in GUARD_PATHS:
            source = ROOT / name
            if source.is_dir():
                shutil.copytree(source, self.repo / name)
            else:
                shutil.copy(source, self.repo / name)
        # A launch root wired the way test_guards.py requires real ones to be,
        # and a deeper directory that has no link of its own.
        self.package = self.repo / "crates" / "pkg"
        self.unlinked = self.package / "src"
        self.unlinked.mkdir(parents=True)
        (self.package / "Cargo.toml").write_text('[package]\nname = "pkg"\n')
        for settings in LINKED_SETTINGS:
            os.symlink(f"../../{settings}", self.package / settings)

    def endpoint(self, script=None):
        fake = FakeEndpoint(script)
        self.addCleanup(fake.close)
        return fake

    def assert_reported_block(self, result, evidence):
        # "No request arrived" also happens when a client simply fails to
        # start. Requiring the client's own mention of the hook rules that out.
        self.assertIn(evidence, result.stdout + result.stderr)

    def run_client(self, arguments, cwd, **environment):
        env = {"PATH": os.environ["PATH"], "HOME": str(self.home), **environment}
        return subprocess.run(
            [self.client, *arguments], cwd=cwd, env=env, text=True,
            stdin=subprocess.DEVNULL, capture_output=True, timeout=120,
        )


class ClaudeCodeLive(LiveCase):
    client = "claude"

    def claude(self, fake, cwd, *arguments):
        return self.run_client(
            ["-p", PROMPT, *arguments], cwd,
            CLAUDE_CONFIG_DIR=str(self.home / ".claude"),
            ANTHROPIC_BASE_URL=fake.url, ANTHROPIC_API_KEY="sk-ant-synthetic",
            CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1", DISABLE_AUTOUPDATER="1",
        )

    def tool_script(self):
        self.marker = self.repo / "MARKER"
        return [
            {"name": "Bash", "input": {
                "command": f"touch {self.marker}; cat {self.repo / 'secret.txt'}",
                "description": "probe"}},
            {"name": "Read", "input": {"file_path": str(self.repo / "secret.txt")}},
            {"name": "Write", "input": {"file_path": str(self.marker), "content": "x"}},
        ]

    def test_harness_detects_an_unguarded_directory(self):
        # Control: without guards the scripted tools run and the secret leaks.
        # If this ever fails, the "blocked" results below prove nothing.
        shutil.rmtree(self.repo / ".claude")
        fake = self.endpoint(self.tool_script())
        self.claude(fake, self.repo, "--dangerously-skip-permissions")
        self.assertTrue(self.marker.exists())
        self.assertIn(SECRET, fake.sent())

    def test_prompt_is_blocked_before_any_request(self):
        for cwd in (self.repo, self.package):
            with self.subTest(cwd=cwd.name):
                fake = self.endpoint()
                result = self.claude(fake, cwd, "--dangerously-skip-permissions")
                self.assertEqual(fake.requests, [])
                self.assert_reported_block(result, "UserPromptSubmit")

    def test_tools_stay_denied_without_the_prompt_hook(self):
        # Defence in depth: delete the prompt hook, switch hooks off through a
        # local override, and ask for bypass mode. The deny list must still
        # keep every scripted tool from running.
        path = self.repo / ".claude" / "settings.json"
        settings = json.loads(path.read_text())
        del settings["hooks"]["UserPromptSubmit"]
        path.write_text(json.dumps(settings))
        (self.repo / ".claude" / "settings.local.json").write_text(json.dumps({
            "disableAllHooks": True,
            "permissions": {"allow": ["Bash", "Read", "Write"],
                            "defaultMode": "bypassPermissions"},
        }))
        fake = self.endpoint(self.tool_script())
        self.claude(fake, self.repo, "--dangerously-skip-permissions")
        self.assertFalse(self.marker.exists())
        self.assertNotIn(SECRET, fake.sent())


class GeminiLive(LiveCase):
    client = "gemini"

    def test_prompt_is_blocked_before_any_request(self):
        for cwd in (self.repo, self.package):
            with self.subTest(cwd=cwd.name):
                fake = self.endpoint()
                result = self.run_client(["--yolo", "-p", PROMPT], cwd,
                                         GEMINI_API_KEY="synthetic",
                                         GOOGLE_GEMINI_BASE_URL=fake.url)
                self.assertEqual(fake.requests, [])
                self.assert_reported_block(result, "Restricted repository")


class CodexLive(LiveCase):
    client = "codex"

    def codex(self, fake, cwd, *arguments):
        (self.home / ".codex").mkdir(exist_ok=True)  # Codex refuses to start without it
        provider = "model_providers.fake"
        return self.run_client(
            ["exec", "--skip-git-repo-check", "-c", "model_provider=fake",
             "-c", f'{provider}.name="fake"',
             "-c", f'{provider}.base_url="{fake.url}/v1"',
             "-c", f'{provider}.env_key="FAKE_KEY"',
             "-c", f"{provider}.request_max_retries=0",
             "-c", f"{provider}.stream_max_retries=0", *arguments, PROMPT],
            cwd, CODEX_HOME=str(self.home / ".codex"), FAKE_KEY="synthetic",
        )

    def trust_project(self):
        (self.home / ".codex").mkdir(exist_ok=True)
        (self.home / ".codex" / "config.toml").write_text(
            f'[projects."{self.repo}"]\ntrust_level = "trusted"\n')

    def test_trusted_hooks_block_the_prompt_even_from_a_subdirectory(self):
        # Codex only runs repository hooks a human has trusted. The flag below
        # stands in for that review; without it the hooks are skipped.
        self.trust_project()
        fake = self.endpoint()
        result = self.codex(fake, self.unlinked, "--dangerously-bypass-hook-trust")
        self.assertEqual(fake.requests, [])
        self.assert_reported_block(result, "UserPromptSubmit")


class OpenCodeLive(LiveCase):
    client = "opencode"

    def test_every_tool_resolves_to_disabled_from_a_subdirectory(self):
        # Checks how opencode resolves the configuration, not a live tool call.
        state = {f"XDG_{kind}_HOME": str(self.home / kind.lower())
                 for kind in ("CONFIG", "DATA", "CACHE", "STATE")}
        result = self.run_client(["debug", "agent", "build"], self.unlinked,
                                 OPENCODE_DISABLE_AUTOUPDATE="1", **state)
        tools = json.loads(result.stdout)["tools"]
        self.assertTrue(tools)
        self.assertEqual({name for name, enabled in tools.items() if enabled}, set())


class KnownGaps(LiveCase):
    """Bypasses that repository files cannot close, kept as executable facts.

    Each test passes while the gap is still open. A failure is good news: the
    client changed, so update the README's list of limitations.
    """

    client = "claude"

    def leaks(self, cwd, *arguments):
        fake = self.endpoint(ClaudeCodeLive.tool_script(self))
        ClaudeCodeLive.claude(self, fake, cwd, "--dangerously-skip-permissions", *arguments)
        return SECRET in fake.sent()

    def test_claude_ignores_guards_in_a_directory_without_a_link(self):
        self.assertTrue(self.leaks(self.unlinked))

    def test_claude_launch_flag_skips_project_settings(self):
        self.assertTrue(self.leaks(self.repo, "--setting-sources", "user"))

    def test_codex_skips_hooks_nobody_has_trusted(self):
        if not shutil.which("codex"):
            self.skipTest("codex is not installed")
        self.client = "codex"
        fake = self.endpoint()
        CodexLive.codex(self, fake, self.repo)
        self.assertNotEqual(fake.requests, [])


if __name__ == "__main__":
    unittest.main()
