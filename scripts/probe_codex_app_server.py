#!/usr/bin/env python3
"""Probe an exact Codex binary with disposable data, a loopback model and stdio MCP.

The default mode uses no login or user configuration. --installed-account instead
checks existing account availability without creating a thread or model turn.
This is opt-in capability tooling, not a production client, model evaluation, or
release qualification. Run without -O.
"""

import argparse
import base64
from contextlib import contextmanager
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
import platform
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading
import time
import uuid

PRIVATE = "CANISEND-SYNTHETIC-PRIVATE"
HOST_SKILL_MARKER = "CANISEND-SYNTHETIC-HOST-SKILL"
TOOL_ARGS = {"confirmed_private_read": True}
FEATURES = """
[features]
shell_tool = false
unified_exec = false
multi_agent = false
shell_snapshot = false
plugins = false
apps = false
hooks = false
skill_mcp_dependency_install = false
enable_request_compression = false
"""


def serve_mcp(marker):
    marker.write_text("started\n")
    for line in sys.stdin:
        message = json.loads(line)
        if "id" not in message:
            continue
        method = message.get("method")
        if method == "initialize":
            result = {
                "protocolVersion": "2025-11-25", "capabilities": {"tools": {}},
                "serverInfo": {"name": "fixture", "version": "1"},
            }
        elif method == "tools/list":
            result = {"tools": [{
                "name": "private_read", "description": "Read only fixture data",
                "inputSchema": {"type": "object", "properties": {
                    "confirmed_private_read": {"type": "boolean"}},
                    "required": ["confirmed_private_read"]},
                "annotations": {"readOnlyHint": True, "destructiveHint": False,
                                "openWorldHint": False},
            }]}
        elif method == "tools/call":
            assert message["params"]["name"] == "private_read"
            assert message["params"]["arguments"] == TOOL_ARGS
            with marker.open("a") as output:
                output.write("called\n")
            result = {"content": [{"type": "text", "text": PRIVATE}]}
        else:
            result = {}
        print(json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result}),
              flush=True)


class LocalModel(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_POST(self):
        size = int(self.headers["Content-Length"])
        if not 0 < size <= 4 * 1024 * 1024:
            self.send_error(413)
            return
        request = json.loads(self.rfile.read(size))
        self.server.host_skill_received = getattr(self.server, "host_skill_received", False) or (
            HOST_SKILL_MARKER in json.dumps(request))
        self.server.tool_names = [tool.get("name", tool["type"])
                                  for tool in request.get("tools", [])]
        # Keep only assertions, never retain provider input/transcripts in the report.
        serialized = json.dumps([item for item in request.get("input", [])
                                 if item.get("type") == "function_call_output"
                                 and item.get("call_id") == self.server.call_id])
        self.server.private_received |= PRIVATE in serialized
        self.server.image_received |= '"type": "input_image"' in serialized
        if self.server.next_call is not None:
            name, arguments, namespace = self.server.next_call
            self.server.next_call = None
            item = {"type": "function_call", "id": "fc_fixture",
                    "call_id": self.server.call_id, "name": name,
                    "arguments": json.dumps(arguments)}
            if namespace:
                item["namespace"] = namespace
        else:
            item = {"type": "message", "id": "msg_fixture", "role": "assistant",
                    "status": "completed", "content": [{"type": "output_text",
                    "text": "Fixture complete.", "annotations": []}]}
        events = [
            {"type": "response.created", "response": {
                "id": "resp_fixture", "status": "in_progress", "output": []}},
            {"type": "response.output_item.added", "output_index": 0, "item": item},
            {"type": "response.output_item.done", "output_index": 0, "item": item},
            {"type": "response.completed", "response": {
                "id": "resp_fixture", "status": "completed", "output": [item],
                "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2}}},
        ]
        body = "".join("event: " + event["type"] + "\ndata: " + json.dumps(event)
                       + "\n\n" for event in events).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class ProbeClient:
    def __init__(self, process):
        self.process = process
        self.messages = queue.Queue(maxsize=128)
        self.pending = []
        self.request_id = 0
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()

    def read(self):
        for line in self.process.stdout:
            if len(line) > 4 * 1024 * 1024:
                break
            self.messages.put(json.loads(line))
        self.messages.put(None)

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def receive(self, deadline):
        message = self.messages.get(timeout=max(0.01, deadline - time.monotonic()))
        assert message is not None, "App Server closed before completing the probe"
        return message

    def request(self, method, params):
        self.request_id += 1
        self.send({"id": self.request_id, "method": method, "params": params})
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            message = self.receive(deadline)
            if message.get("id") == self.request_id and "method" not in message:
                return message
            self.pending.append(message)
        raise TimeoutError("App Server request deadline")


@contextmanager
def client(binary, config_dir, session, overrides=(), host_home=None):
    # Configure CODEX_HOME only in this child's environment, for its intended purpose.
    # Do not copy auth files or user-global configuration. The explicit account-only
    # mode leaves the installed config home selected; it never starts a thread/turn.
    environment = {"PATH": os.environ.get("PATH", os.defpath)}
    if config_dir is not None:
        environment["CODEX_HOME"] = str(config_dir)
    elif "CODEX_HOME" in os.environ:
        environment["CODEX_HOME"] = os.environ["CODEX_HOME"]
    if host_home is not None:
        environment.update({"HOME": str(host_home), "USERPROFILE": str(host_home),
                            "XDG_CONFIG_HOME": str(host_home / "config"),
                            "XDG_DATA_HOME": str(host_home / "data"),
                            "APPDATA": str(host_home / "data"),
                            "LOCALAPPDATA": str(host_home / "local")})
    arguments = [str(binary), "app-server", "--listen", "stdio://"]
    for override in overrides:
        arguments.extend(["-c", override])
    process = subprocess.Popen(
        arguments, cwd=session,
        env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True,
    )
    probe = ProbeClient(process)
    try:
        initialized = probe.request("initialize", {
            "clientInfo": {"name": "canisend-capability-probe", "version": "1"},
            "capabilities": {"experimentalApi": True},
        })
        assert "result" in initialized, "initialize failed"
        if config_dir is not None:
            assert Path(initialized["result"]["codexHome"]).resolve() == config_dir.resolve()
        probe.send({"method": "initialized"})
        yield probe
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        process.stdin.close()
        probe.reader.join(timeout=2)
        process.stdout.close()


def called(marker):
    return marker.read_text().count("called\n") if marker.exists() else 0


def turn(probe, model, thread_id, marker, action, policy="on-request", call=None,
         permissions=None):
    model.private_received = model.image_received = False
    model.call_counter += 1
    model.call_id = f"call_fixture_{model.call_counter}"
    model.next_call = call or ("private_read", TOOL_ARGS, "mcp__fixture")
    before = called(marker)
    response = probe.request("turn/start", {
        "threadId": thread_id, "input": [{"type": "text", "text": "Run the local fixture."}],
        "approvalPolicy": policy, "approvalsReviewer": "user",
        **({"permissions": permissions} if permissions else {
            "sandboxPolicy": {"type": "readOnly", "networkAccess": False}}),
    })
    assert "result" in response, "turn/start failed"
    turn_id = response["result"]["turn"]["id"]
    approvals = 0
    active = {}
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        message = probe.pending.pop(0) if probe.pending else probe.receive(deadline)
        method = message.get("method")
        params = message.get("params", {})
        if method == "item/started" and params["item"]["type"] == "mcpToolCall":
            active[params["item"]["id"]] = params["item"]
        if "id" in message and method:
            assert method == "mcpServer/elicitation/request", "unexpected approval method"
            assert params["threadId"] == thread_id and params["turnId"] == turn_id
            assert params["serverName"] == "fixture" and params["mode"] == "form"
            assert params["_meta"]["codex_approval_kind"] == "mcp_tool_call"
            assert params["_meta"]["tool_params"] == TOOL_ARGS
            assert "itemId" not in params  # Missing in this exact provider protocol.
            matches = [item for item in active.values() if item["server"] == "fixture"
                       and item["tool"] == "private_read" and item["arguments"] == TOOL_ARGS]
            assert len(matches) == 1, "fixture approval is not uniquely correlated"
            assert called(marker) == before and not model.private_received
            approvals += 1
            probe.send({"id": message["id"], "result": {
                "action": action, "content": {} if action == "accept" else None}})
        if method == "item/completed":
            active.pop(params["item"]["id"], None)
        if method == "turn/completed" and params["turn"]["id"] == turn_id:
            assert params["turn"]["status"] == "completed", "fixture turn failed"
            return {"approvals": approvals, "calls": called(marker) - before,
                    "private_received": model.private_received,
                    "image_received": model.image_received,
                    "tool_names": model.tool_names}
    raise TimeoutError("App Server turn deadline")


def bootstrap_overrides(binary, session, profile):
    """Exercise the desktop's process-level policy recipe against the real provider."""
    filesystem = ('{ ":minimal" = "read", '
                  + json.dumps(str(session.resolve())) + ' = "read", '
                  + json.dumps(str(binary.parent)) + ' = "read" }')
    return [f"default_permissions={json.dumps(profile)}",
            f"permissions.{profile}.filesystem={filesystem}",
            f"permissions.{profile}.network.enabled=false",
            'approval_policy="never"', 'approvals_reviewer="user"',
            'web_search="disabled"',
            *(f"features.{name}=false" for name in [
                "shell_tool", "unified_exec", "multi_agent", "shell_snapshot",
                "plugins", "apps", "hooks", "skill_mcp_dependency_install"])]


def probe_account(binary):
    """Read only account availability; no thread, model turn, or token refresh."""
    with tempfile.TemporaryDirectory(prefix="canisend-account-probe-") as temporary:
        profile = "canisend_" + uuid.uuid4().hex
        overrides = bootstrap_overrides(binary, Path(temporary), profile)
        with client(binary, None, Path(temporary), overrides) as probe:
            response = probe.request("account/read", {"refreshToken": False})
            assert "result" in response, "account/read failed"
            account = response["result"]
            return {"initialize": True,
                    "requires_openai_auth": account.get("requiresOpenaiAuth"),
                    "existing_account_available": account.get("account") is not None,
                    "model_turns": 0, "refresh_requested": False}


def probe_binary(binary):
    with tempfile.TemporaryDirectory(prefix="canisend-codex-version-") as version_dir:
        version = subprocess.check_output(
            [str(binary), "--version"], text=True,
            env={"PATH": os.environ.get("PATH", os.defpath), "CODEX_HOME": version_dir},
        ).strip()
    assert version == "codex-cli 0.152.0", "requalify assertions for this exact CLI version"
    with binary.open("rb") as executable:
        digest = hashlib.file_digest(executable, "sha256").hexdigest()
    report = {"version": version, "binary_sha256": digest,
              "platform": sys.platform, "architecture": platform.machine(), "checks": {},
              "embedded_enablement": False, "authentication_reuse": "not tested"}
    with tempfile.TemporaryDirectory(prefix="canisend-codex-probe-") as temporary:
        root = Path(temporary).resolve()
        config_dir, session = root / "provider", root / "session"
        config_dir.mkdir()
        session.mkdir()
        marker = root / "mcp.marker"
        model = ThreadingHTTPServer(("127.0.0.1", 0), LocalModel)
        model.next_call = None
        model.call_counter = 0
        model.call_id = "none"
        model.private_received = model.image_received = False
        model.tool_names = []
        threading.Thread(target=model.serve_forever, daemon=True).start()
        base = f'''model_provider = "fixture"
model = "fixture"
web_search = "disabled"
[model_providers.fixture]
name = "local fixture"
base_url = "http://127.0.0.1:{model.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
{FEATURES}
[mcp_servers.fixture]
command = {json.dumps(sys.executable)}
args = {json.dumps([str(Path(__file__).resolve()), "--mcp", str(marker)])}
required = true
default_tools_approval_mode = "prompt"
startup_timeout_sec = 2
'''
        config_file = config_dir / "config.toml"
        config_file.write_text(base)
        setup = {"cwd": str(session), "sandbox": "read-only", "approvalPolicy": "never",
                 "approvalsReviewer": "user", "ephemeral": False}
        missing = '\n[mcp_servers.unrelated]\ncommand = "/canisend-fixture-missing"\nrequired = true\n'
        try:
            with client(binary, config_dir, session) as probe:
                started = probe.request("thread/start", setup)
                assert "result" in started, "required fixture MCP did not initialize"
                thread_id = started["result"]["thread"]["id"]
                for action, policy in [("decline", "never"), ("decline", "on-request"),
                                       ("accept", "on-request"), ("decline", "on-request")]:
                    result = turn(probe, model, thread_id, marker, action, policy)
                    allowed = action == "accept"
                    assert result["calls"] == int(allowed)
                    assert result["private_received"] == allowed
                    assert result["approvals"] == int(policy != "never")
                    report["checks"][f"{policy}-{action}-{called(marker)}"] = result
                assert "exec_command" not in model.tool_names
                assert "write_stdin" not in model.tool_names
                assert "multi_agent_v1" not in model.tool_names
                report["checks"]["shell_and_agent_tools_removed"] = True
                # A real image outside the session proves the remaining read path.
                outside = root / "outside.png"
                outside.write_bytes(base64.b64decode(
                    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGMQVDIGAACuAGcVHqFfAAAAAElFTkSuQmCC"))
                image_result = turn(probe, model, thread_id, marker, "decline", call=(
                    "view_image", {"path": str(outside)}, None))
                assert image_result["image_received"], "legacy read counterexample did not execute"
                report["checks"]["legacy_read_only_outside_image"] = image_result
            # Resume must prompt again; no earlier accept grants a session-wide read.
            with client(binary, config_dir, session) as probe:
                resumed = probe.request("thread/resume", {
                    **{key: value for key, value in setup.items() if key != "ephemeral"},
                    "threadId": thread_id, "excludeTurns": True})
                assert "result" in resumed, "thread/resume failed"
                result = turn(probe, model, thread_id, marker, "decline")
                assert result["approvals"] == 1 and result["calls"] == 0
                report["checks"]["resume_decline"] = result
            unrelated_marker = root / "unrelated.marker"
            unrelated = f'\n[mcp_servers.unrelated]\ncommand = {json.dumps(sys.executable)}\n'
            unrelated += "args = " + json.dumps([
                str(Path(__file__).resolve()), "--mcp", str(unrelated_marker)]) + "\nrequired = true\n"
            config_file.write_text(base + unrelated)
            for disable in [False, True]:
                unrelated_marker.unlink(missing_ok=True)
                with client(binary, config_dir, session) as probe:
                    assert not unrelated_marker.exists(), "inherited MCP started during initialize"
                    override = {"unrelated": {"enabled": False}} if disable else {}
                    result = probe.request("thread/start", {
                        **setup, "config": {"mcp_servers": override}})
                    assert "result" in result
                    assert unrelated_marker.exists() != disable
                    report["checks"]["inherited_mcp_explicit_disable_" + str(disable).lower()] = True
            config_file.write_text(base + missing)
            with client(binary, config_dir, session) as probe:
                for method in ["thread/start", "thread/resume"]:
                    params = {**setup, "config": {"mcp_servers": {}}}
                    if method == "thread/resume":
                        params.pop("ephemeral")
                        params["threadId"] = thread_id
                    failed = probe.request(method, params)
                    assert "required MCP servers failed" in failed.get("error", {}).get("message", "")
                    report["checks"][method + "_required_failure_empty_override"] = True
            # A read-time inventory cannot seal the inherited server set: creation reloads it.
            config_file.write_text(base + missing)
            with client(binary, config_dir, session) as probe:
                inventory = probe.request("config/read", {"includeLayers": False})
                names = inventory["result"]["config"]["mcp_servers"]
                assert "added_after_read" not in names
                with config_file.open("a") as output:
                    output.write('\n[mcp_servers.added_after_read]\n'
                                 'command="/canisend-missing-fixture"\nrequired=true\n')
                raced = probe.request("thread/start", {**setup, "config": {
                    "mcp_servers": {name: {"enabled": False} for name in names}}})
                assert "added_after_read" in raced.get("error", {}).get("message", "")
                report["checks"]["inherited_mcp_reloaded_after_inventory"] = True

            # Probe the same literal-directory, pre-initialize overrides used by the desktop.
            overrides = bootstrap_overrides(binary, session, "canisend")
            inherited = base
            for setting in overrides:
                if setting.startswith("features."):
                    name = setting.removeprefix("features.").split("=", 1)[0]
                    inherited = inherited.replace(name + " = false", name + " = true")
            config_file.write_text(inherited)
            with client(binary, config_dir, session, overrides) as probe:
                params = {key: value for key, value in setup.items() if key != "sandbox"}
                restricted = probe.request("thread/start", {**params, "permissions": "canisend"})
                assert "result" in restricted, "restricted profile failed before tool use"
                report["checks"]["restricted_profile_started"] = True
                restricted_id = restricted["result"]["thread"]["id"]
                inside = session / "inside.png"
                inside.write_bytes(outside.read_bytes())
                for label, path in [("inside", inside), ("outside", outside)]:
                    result = turn(probe, model, restricted_id, marker, "decline",
                                  call=("view_image", {"path": str(path)}, None),
                                  permissions="canisend")
                    assert result["image_received"] == (label == "inside")
                    assert result["approvals"] == 0
                    report["checks"]["restricted_image_" + label] = result
                result = turn(probe, model, restricted_id, marker, "accept",
                              permissions="canisend")
                assert result["approvals"] == 1 and result["calls"] == 1
                assert result["private_received"]
                report["checks"]["restricted_consented_mcp"] = result
                # Dispatch a disabled built-in against fixture data. Absence in the catalog
                # alone is not proof that a model-supplied call will be rejected.
                result = turn(probe, model, restricted_id, marker, "decline",
                              call=("exec_command", {"cmd": "echo " + PRIVATE}, None),
                              permissions="canisend")
                assert not result["private_received"] and result["approvals"] == 0
                report["checks"]["restricted_command_rejected"] = result
            resume_profile = "canisend-resumed"
            with client(binary, config_dir, session,
                        bootstrap_overrides(binary, session, resume_profile)) as probe:
                resumed = probe.request("thread/resume", {
                    **{key: value for key, value in params.items() if key != "ephemeral"},
                    "permissions": resume_profile, "threadId": restricted_id, "excludeTurns": True})
                assert "result" in resumed, "restricted resume failed"
                result = turn(probe, model, restricted_id, marker, "decline",
                              call=("view_image", {"path": str(outside)}, None),
                              permissions=resume_profile)
                assert not result["image_received"]
                report["checks"]["restricted_resume_outside_denied"] = result

            # A disposable standalone skill proves whether the host-discovery switch affects
            # model-visible context, with and without an explicit directory read grant.
            skill = config_dir / "skills" / "canisend-host-fixture"
            skill.mkdir(parents=True)
            (skill / "SKILL.md").write_text(
                '---\nname: canisend-host-fixture\ndescription: '
                + HOST_SKILL_MARKER + '\n---\nLocal fixture only.\n')
            for skip, allow_skill in [(False, True), (True, True), (True, False)]:
                overrides = bootstrap_overrides(binary, session, "canisend")
                if allow_skill:
                    overrides[1] = overrides[1][:-2] + ', ' + json.dumps(str(skill)) + ' = "read" }'
                overrides.append(f"features.skip_host_skill_discovery={str(skip).lower()}")
                with client(binary, config_dir, session, overrides) as probe:
                    effective = probe.request("config/read", {"includeLayers": False})
                    assert effective["result"]["config"]["features"]["skip_host_skill_discovery"] == skip
                    started = probe.request("thread/start", {**params, "permissions": "canisend"})
                    assert "result" in started, "host skill fixture startup failed"
                    model.host_skill_received = False
                    turn(probe, model, started["result"]["thread"]["id"], marker,
                         "decline", permissions="canisend")
                    # Record the counterexample: neither the skip flag nor omitting the
                    # directory read grant suppresses this standalone skill's metadata.
                    assert model.host_skill_received, (
                        f"host skill visibility mismatch: skip={skip}, allow={allow_skill}, "
                        f"received={model.host_skill_received}")
                    report["checks"][f"host_skill_skip_{skip}_read_{allow_skill}"] = {
                        "skip_flag": skip, "read_allowed": allow_skill,
                        "model_received_fixture_metadata": model.host_skill_received}

            # Keep the original standalone fixture as a counterexample, but remove it from
            # this dedicated provider directory before checking external-home isolation.
            (skill / "SKILL.md").unlink()
            skill.rmdir()
            host_home, isolated_home = root / "host", root / "isolated-host"
            isolated_home.mkdir()
            skill = host_home / ".agents" / "skills" / "canisend-host-fixture"
            skill.mkdir(parents=True)
            (skill / "SKILL.md").write_text(
                '---\nname: canisend-host-fixture\ndescription: '
                + HOST_SKILL_MARKER + '\n---\nLocal fixture only.\n')
            (host_home / ".codex").mkdir()
            host_config = host_home / ".codex" / "config.toml"
            host_config.write_text(missing)
            for host in [host_home, isolated_home]:
                dedicated_config = host / "provider"
                dedicated_config.mkdir()
                (dedicated_config / "config.toml").write_text(base)
                with client(binary, dedicated_config, session,
                            bootstrap_overrides(binary, session, "canisend"), host_home=host) as probe:
                    # Changing external configuration after initialize cannot affect this provider home.
                    host_config.write_text(missing + '\n[mcp_servers.late]\ncommand="/canisend-missing-late"\nrequired=true\n')
                    started = probe.request("thread/start", {**params, "permissions": "canisend"})
                    assert "result" in started, "dedicated provider loaded external MCP configuration"
                    model.host_skill_received = False
                    turn(probe, model, started["result"]["thread"]["id"], marker,
                         "decline", permissions="canisend")
                    assert model.host_skill_received == (host == host_home), "unexpected external Skill visibility"
                    report["checks"]["external_home_skill_visible" if host == host_home
                                     else "dedicated_home_excludes_external_skill_and_mcp"] = {
                        "model_received_fixture_metadata": model.host_skill_received,
                        "external_required_mcp_ignored": True}
                    if host == isolated_home:
                        private_image = dedicated_config / "private-fixture.png"
                        private_image.write_bytes(outside.read_bytes())
                        for label, path, allowed in [("inside_read", inside, True),
                                                     ("provider_image_denied", private_image, False)]:
                            result = turn(probe, model, started["result"]["thread"]["id"], marker,
                                          "decline", call=("view_image", {"path": str(path)}, None),
                                          permissions="canisend")
                            assert result["image_received"] == allowed, "dedicated HOME changed read boundary"
                            report["checks"]["dedicated_home_" + label] = result

        finally:
            model.shutdown()
            model.server_close()
    return report


if __name__ == "__main__":
    if not __debug__:
        raise SystemExit("Run this probe without -O; its assertions must remain enabled.")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--codex", type=Path, help="exact installed Codex executable")
    parser.add_argument("--mcp", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--installed-account", action="store_true",
                        help="only check installed account availability; no model requests")
    args = parser.parse_args()
    if args.mcp:
        serve_mcp(args.mcp)
    else:
        if not args.codex:
            parser.error("--codex is required")
        check = probe_account if args.installed_account else probe_binary
        print(json.dumps(check(args.codex.resolve()), indent=2))
