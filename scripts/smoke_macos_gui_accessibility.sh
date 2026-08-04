#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 CanISend.app" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI accessibility smoke: this script must run on macOS" >&2
  exit 1
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
source "$script_dir/lib/native_paths.sh"
app="$(canisend_absolute_path "$1")"
manifest="$app.manifest.json"
host="$app/Contents/MacOS/canisend-gui"
for command in jq open osascript; do
  command -v "$command" >/dev/null
done
if [[ ! -d "$app" || -L "$app" ]]; then
  echo "macOS GUI accessibility smoke: app must be a regular directory: $app" >&2
  exit 1
fi
if [[ ! -f "$host" || -L "$host" ]]; then
  echo "macOS GUI accessibility smoke: unified host is missing: $host" >&2
  exit 1
fi

"$script_dir/verify_macos_gui_app.sh" "$app" "$manifest"

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-gui-accessibility.XXXXXX")"
keep_fixture="${CANISEND_KEEP_GUI_ACCESSIBILITY_FIXTURE:-0}"
gui_pid=""
launcher_pid=""
cleanup() {
  if [[ -n "$gui_pid" ]] && kill -0 "$gui_pid" 2>/dev/null; then
    kill "$gui_pid" 2>/dev/null || true
  fi
  if [[ -n "$launcher_pid" ]] && kill -0 "$launcher_pid" 2>/dev/null; then
    kill "$launcher_pid" 2>/dev/null || true
    wait "$launcher_pid" 2>/dev/null || true
  fi
  if [[ "$keep_fixture" == "1" ]]; then
    echo "macOS GUI accessibility smoke: preserved fixture at $fixture_root" >&2
  else
    rm -rf "$fixture_root"
  fi
}
trap cleanup EXIT
if [[ "$keep_fixture" == "1" ]]; then
  echo "macOS GUI accessibility smoke: fixture at $fixture_root" >&2
fi
mkdir -p "$fixture_root/home"
mkdir -p "$fixture_root/home/.local/share/mise/shims"
cp "$repo_root/fixtures/runtime/fake-codex-runtime.sh" \
  "$fixture_root/home/.local/share/mise/shims/codex"
chmod 700 "$fixture_root/home/.local/share/mise/shims/codex"
workspace="$fixture_root/workspace"
"$host" --workspace "$workspace" workspace init --pack academic-job --json >/dev/null
workspace="$(CDPATH= cd -- "$workspace" && pwd -P)"
registry="$fixture_root/home/Library/Application Support/CanISend/workspaces.json"
mkdir -p "$(dirname "$registry")"
jq -n \
  --arg workspace "$workspace" \
  '{
    format: "canisend.workspace-registry/v1",
    default_path: $workspace,
    entries: [{
      alias: "Accessibility smoke",
      path: $workspace,
      pinned: false,
      last_opened_unix: 1
    }]
  }' > "$registry"

open -n -W \
  --env "HOME=$fixture_root/home" \
  --env "PATH=$fixture_root/home/.local/share/mise/shims:/usr/bin:/bin" \
  --stdout "$fixture_root/gui.log" \
  --stderr "$fixture_root/gui.log" \
  "$app" &
launcher_pid="$!"

if ! gui_pid="$(osascript - <<'APPLESCRIPT'
tell application "System Events"
    repeat 300 times
        set guiProcesses to every application process whose bundle identifier is "io.github.jxpeng98.canisend"
        if (count of guiProcesses) is 1 then return unix id of item 1 of guiProcesses
        delay 0.1
    end repeat
end tell
error "GUI process did not appear uniquely" number 1
APPLESCRIPT
)"; then
  echo "macOS GUI accessibility smoke: packaged app did not launch" >&2
  sed -n '1,160p' "$fixture_root/gui.log" >&2
  exit 1
fi

if ! osascript - "$gui_pid" <<'APPLESCRIPT'
on assertCondition(conditionValue, failureMessage)
    if conditionValue is false then error failureMessage number 1
end assertCondition

on findNamed(parentElement, targetName)
    tell application "System Events"
        try
            if (name of parentElement as text) is targetName then return parentElement
        end try
        try
            set allElements to entire contents of parentElement
            repeat with childElement in allElements
                try
                    set candidateElement to contents of childElement
                    if (name of candidateElement as text) is targetName then return candidateElement
                end try
            end repeat
        end try
    end tell
    return missing value
end findNamed

on findNamedRole(parentElement, targetName, targetRole)
    tell application "System Events"
        try
            if (name of parentElement as text) is targetName and (value of attribute "AXRole" of parentElement as text) is targetRole then return parentElement
        end try
        try
            set allElements to entire contents of parentElement
            repeat with childElement in allElements
                try
                    set candidateElement to contents of childElement
                    if (name of candidateElement as text) is targetName and (value of attribute "AXRole" of candidateElement as text) is targetRole then return candidateElement
                end try
            end repeat
        end try
    end tell
    return missing value
end findNamedRole

on findValued(parentElement, targetValue)
    tell application "System Events"
        try
            if (value of parentElement as text) is targetValue then return parentElement
        end try
        try
            set allElements to entire contents of parentElement
            repeat with childElement in allElements
                try
                    set candidateElement to contents of childElement
                    if (value of candidateElement as text) is targetValue then return candidateElement
                end try
            end repeat
        end try
    end tell
    return missing value
end findValued

on run arguments
    set guiPid to item 1 of arguments as integer
    tell application "System Events"
        set guiProcess to missing value
        repeat 300 times
            if exists (first process whose unix id is guiPid) then
                set guiProcess to first process whose unix id is guiPid
                if (count of windows of guiProcess) > 0 then exit repeat
            end if
            delay 0.1
        end repeat
        my assertCondition(guiProcess is not missing value, "GUI process did not appear")
        tell guiProcess
            set windowCount to count of windows
            my assertCondition(windowCount is 1, "expected one CanISend window, got " & windowCount)
            set frontmost to true
            set appWindow to window 1
            log "accessibility smoke: window ready"

            set navigationElement to missing value
            repeat 20 times
                set navigationElement to my findNamed(appWindow, "Primary navigation")
                if navigationElement is not missing value then exit repeat
                set switchToEnglish to my findNamed(appWindow, "English")
                if switchToEnglish is not missing value then
                    click switchToEnglish
                    delay 0.4
                end if
                delay 0.1
            end repeat
            my assertCondition(navigationElement is not missing value, "navigation landmark missing")
            my assertCondition((value of attribute "AXRole" of navigationElement as text) is "AXGroup", "navigation role mismatch")

            set todayControl to my findNamed(appWindow, "Today")
            my assertCondition(todayControl is not missing value, "Today navigation control missing")
            click todayControl
            delay 0.3

            set mainElement to my findNamed(appWindow, "CanISend main content")
            my assertCondition(mainElement is not missing value, "main content landmark missing")

            set headingElement to my findValued(appWindow, "Prepare stronger applications with a calmer workflow.")
            my assertCondition(headingElement is not missing value, "Today heading missing")
            set headingParent to value of attribute "AXParent" of headingElement
            my assertCondition((value of attribute "AXRole" of headingParent as text) is "AXHeading", "heading role mismatch")
            my assertCondition((value of headingParent as integer) is 1, "heading level mismatch")

            set settingsControl to my findNamed(appWindow, "Settings")
            my assertCondition(settingsControl is not missing value, "Settings navigation control missing")
            click settingsControl
            log "accessibility smoke: semantics passed"
        end tell
        delay 0.4
        tell guiProcess
            set settingsHeading to my findValued(appWindow, "Settings and diagnostics")
            my assertCondition(settingsHeading is not missing value, "Settings heading missing")

            set pathControl to missing value
            repeat 40 times
                set pathControl to my findNamedRole(appWindow, "Add to PATH", "AXButton")
                if pathControl is not missing value then exit repeat
                delay 0.1
            end repeat
            my assertCondition(pathControl is not missing value, "automatic CLI status did not expose Add to PATH")
            set terminalConsent to my findNamedRole(appWindow, "I confirm CanISend may modify this explicit terminal executable destination.", "AXCheckBox")
            my assertCondition(terminalConsent is not missing value, "terminal mutation consent control missing")
            click terminalConsent
            set installControl to my findNamedRole(appWindow, "Install or upgrade CLI", "AXButton")
            my assertCondition(installControl is not missing value, "CLI install action missing")
            my assertCondition((value of attribute "AXEnabled" of installControl as boolean) is true, "CLI install action is disabled")
            click installControl
        end tell
        set installControl to missing value
        repeat 100 times
            delay 0.1
            tell guiProcess
                set installControl to my findNamedRole(appWindow, "Install or upgrade CLI", "AXButton")
            end tell
            if installControl is not missing value and (value of attribute "AXEnabled" of installControl as boolean) is true then exit repeat
        end repeat
        my assertCondition(installControl is not missing value, "CLI install action disappeared")
        my assertCondition((value of attribute "AXEnabled" of installControl as boolean) is true, "CLI install action did not complete")
        tell guiProcess
            set pathControl to my findNamedRole(appWindow, "Add to PATH", "AXButton")
            my assertCondition(pathControl is not missing value, "Add to PATH action missing after CLI installation")
            my assertCondition((value of attribute "AXEnabled" of pathControl as boolean) is true, "Add to PATH action is disabled after CLI installation")
            click pathControl
            log "accessibility smoke: GUI-managed CLI installation and PATH action passed"
        end tell
        delay 0.5
        tell guiProcess
            set agentControl to my findNamedRole(appWindow, "Agent integration", "AXButton")
            my assertCondition(agentControl is not missing value, "Agent navigation control missing")
            click agentControl
            log "accessibility smoke: Agent navigation passed"
        end tell
        delay 0.5
        tell guiProcess
            set externalHostControl to my findNamed(appWindow, "Agent host")
            my assertCondition(externalHostControl is not missing value, "external Agent host tab is not the default surface")
            set prepareMcpControl to my findNamedRole(appWindow, "Prepare MCP configuration", "AXButton")
            my assertCondition(prepareMcpControl is not missing value, "MCP configuration action missing")
            my assertCondition((value of attribute "AXEnabled" of prepareMcpControl as boolean) is true, "MCP configuration action is disabled")
            click prepareMcpControl
            log "accessibility smoke: MCP configuration action passed"
        end tell
        set readOnlyToolCount to missing value
        set guardedWriteToolCount to missing value
        repeat 40 times
            delay 0.1
            tell guiProcess
                set readOnlyToolCount to my findValued(appWindow, "13 read-only / preview tools")
                set guardedWriteToolCount to my findValued(appWindow, "9 approval-gated writes")
            end tell
            if readOnlyToolCount is not missing value and guardedWriteToolCount is not missing value then exit repeat
        end repeat
        my assertCondition(readOnlyToolCount is not missing value, "MCP read-only/preview tool count missing")
        my assertCondition(guardedWriteToolCount is not missing value, "MCP approval-gated write count missing")
        log "accessibility smoke: external-first MCP permission categories passed"
        tell guiProcess
            set inAppControl to missing value
            repeat 40 times
                set inAppControl to my findNamed(appWindow, "In-App read-only")
                if inAppControl is not missing value then exit repeat
                delay 0.1
            end repeat
            my assertCondition(inAppControl is not missing value, "in-App Agent bridge tab missing")
            click inAppControl
        end tell
        delay 0.4
        tell guiProcess
            set agentMessage to my findNamedRole(appWindow, "Conversation", "AXTextArea")
            my assertCondition(agentMessage is not missing value, "Agent message control missing")
            set runtimeDetectedStatus to my findValued(appWindow, "CLI detected")
            my assertCondition(runtimeDetectedStatus is not missing value, "runtime evidence did not report CLI detection")
            set sessionBindingStatus to my findValued(appWindow, "External session ID binding")
            my assertCondition(sessionBindingStatus is not missing value, "runtime evidence did not report the external session binding strategy")
            set runtimeEvidence to my findValued(appWindow, "Detection proves only the executable path and version. Sign-in, search, MCP, skills, and plugins are not inspected by CanISend and are confirmed only when the host runs.")
            my assertCondition(runtimeEvidence is not missing value, "runtime evidence overclaimed authentication or host configuration")
            set inferredSignInStatus to my findValued(appWindow, "Local sign-in")
            my assertCondition(inferredSignInStatus is missing value, "runtime discovery incorrectly inferred local sign-in")
            click agentMessage
            set value of agentMessage to "Wait for the local cancellation fixture."
            set providerConsent to my findNamedRole(appWindow, "I confirm this local runtime may read the selected workspace and send necessary context to its configured provider. The host stores its own transcript.", "AXCheckBox")
            my assertCondition(providerConsent is not missing value, "Agent provider consent control missing")
            if (value of providerConsent as boolean) is false then click providerConsent
            my assertCondition((value of providerConsent as boolean) is true, "Agent provider consent did not enable")
            set sendControl to missing value
            repeat 40 times
                set sendControl to my findNamedRole(appWindow, "Send message", "AXButton")
                if sendControl is not missing value and (value of attribute "AXEnabled" of sendControl as boolean) is true then exit repeat
                delay 0.1
            end repeat
            my assertCondition(sendControl is not missing value, "Agent send action missing")
            my assertCondition((value of attribute "AXEnabled" of sendControl as boolean) is true, "Agent runtime did not make the send action available")
            click sendControl
        end tell
        log "accessibility smoke: bounded runtime evidence passed"
        set cancelTurnControl to missing value
        repeat 40 times
            delay 0.1
            tell guiProcess
                set cancelTurnControl to my findNamedRole(appWindow, "Cancel turn", "AXButton")
            end tell
            if cancelTurnControl is not missing value then exit repeat
        end repeat
        my assertCondition(cancelTurnControl is not missing value, "Agent cancel action did not appear")
        tell guiProcess
            click cancelTurnControl
        end tell
        set cancellationNotice to missing value
        repeat 40 times
            delay 0.1
            tell guiProcess
                set cancellationNotice to my findValued(appWindow, "The local Agent turn was cancelled. No partial response was saved.")
            end tell
            if cancellationNotice is not missing value then exit repeat
        end repeat
        my assertCondition(cancellationNotice is not missing value, "Agent cancellation did not complete")
        log "accessibility smoke: scoped Agent turn cancellation passed"
        set newConversationControl to missing value
        repeat 40 times
            delay 0.1
            tell guiProcess
                set newConversationControl to my findNamedRole(appWindow, "New conversation", "AXButton")
            end tell
            if newConversationControl is not missing value and (value of attribute "AXEnabled" of newConversationControl as boolean) is true then exit repeat
        end repeat
        my assertCondition(newConversationControl is not missing value, "New Agent conversation action did not return after cancellation")
        my assertCondition((value of attribute "AXEnabled" of newConversationControl as boolean) is true, "Agent scope remained busy after cancellation")
        tell guiProcess
            click newConversationControl
            set agentMessage to my findNamedRole(appWindow, "Conversation", "AXTextArea")
            click agentMessage
            set value of agentMessage to "Complete the local session fixture."
            set sendControl to my findNamedRole(appWindow, "Send message", "AXButton")
            my assertCondition(sendControl is not missing value, "Agent send action missing after starting a new conversation")
            my assertCondition((value of attribute "AXEnabled" of sendControl as boolean) is true, "Agent send action is disabled after starting a new conversation")
            click sendControl
        end tell
        set firstTurnResponse to missing value
        repeat 40 times
            delay 0.1
            tell guiProcess
                set firstTurnResponse to my findValued(appWindow, "Fixture first turn completed.")
            end tell
            if firstTurnResponse is not missing value then exit repeat
        end repeat
        my assertCondition(firstTurnResponse is not missing value, "successful Agent fixture turn did not complete")
        log "accessibility smoke: first Agent session turn passed"
        tell guiProcess
            set settingsControl to my findNamedRole(appWindow, "Settings", "AXButton")
            my assertCondition(settingsControl is not missing value, "Settings navigation control missing after Agent completion")
            click settingsControl
        end tell
        delay 0.4
        tell guiProcess
            set appearanceTab to my findNamed(appWindow, "Appearance")
            my assertCondition(appearanceTab is not missing value, "Appearance tab missing")
            click appearanceTab
        end tell
        delay 0.3

        tell guiProcess
            set appearanceHeading to my findValued(appWindow, "Accessibility & appearance")
            my assertCondition(appearanceHeading is not missing value, "accessibility and appearance heading missing")
            set languageControl to my findNamedRole(appWindow, "Language", "AXPopUpButton")
            my assertCondition(languageControl is not missing value, "language control missing")
            set textSizeControl to my findNamedRole(appWindow, "Text size", "AXPopUpButton")
            my assertCondition(textSizeControl is not missing value, "text size control missing")
            set motionControl to my findNamed(appWindow, "Reduce motion")
            my assertCondition(motionControl is not missing value, "Reduce motion control missing")
            if (value of motionControl as boolean) is false then click motionControl
            my assertCondition((value of motionControl as boolean) is true, "Reduce motion did not enable")
            log "accessibility smoke: reduced motion passed"
        end tell

        tell guiProcess to set frontmost to true
        delay 0.1
        repeat 10 times
            keystroke "=" using command down
            delay 0.05
        end repeat
        delay 0.3
        tell guiProcess
            set textSizeControl to my findNamedRole(appWindow, "Text size", "AXPopUpButton")
            my assertCondition((value of textSizeControl as text) is "200%", "200% text size did not apply")
            log "accessibility smoke: 200% text size passed"
        end tell

        tell guiProcess to set frontmost to true
        keystroke "0" using command down
        delay 0.3
        tell guiProcess
            set textSizeControl to my findNamedRole(appWindow, "Text size", "AXPopUpButton")
            my assertCondition((value of textSizeControl as text) is "100%", "Command-0 did not restore 100% text size")
            log "accessibility smoke: 100% reset passed"
        end tell

        tell guiProcess
            set languageControl to my findNamedRole(appWindow, "Language", "AXPopUpButton")
            click languageControl
        end tell
        delay 0.2
        key code 125
        key code 36
        delay 0.5
        tell guiProcess
            set languageControl to my findNamedRole(appWindow, "语言", "AXPopUpButton")
            my assertCondition(languageControl is not missing value, "Chinese language control missing")
            my assertCondition((value of languageControl as text) is "简体中文", "Chinese language selection did not persist in the UI")
            log "accessibility smoke: Simplified Chinese locale and native control names passed"
        end tell

        keystroke "q" using command down
        log "accessibility smoke: quit requested"
    end tell
end run
APPLESCRIPT
then
  echo "macOS GUI accessibility smoke: Accessibility automation failed" >&2
  sed -n '1,160p' "$fixture_root/gui.log" >&2
  echo "macOS GUI accessibility smoke: synthetic profile state after automation failure" >&2
  "$host" --workspace "$workspace" profile source list --json >&2 || true
  exit 1
fi

wait "$launcher_pid"
gui_pid=""
launcher_pid=""

open -n -W \
  --env "HOME=$fixture_root/home" \
  --env "PATH=$fixture_root/home/.local/share/mise/shims:/usr/bin:/bin" \
  --stdout "$fixture_root/restart-gui.log" \
  --stderr "$fixture_root/restart-gui.log" \
  "$app" &
launcher_pid="$!"
if ! gui_pid="$(osascript - <<'APPLESCRIPT'
tell application "System Events"
    repeat 300 times
        set guiProcesses to every application process whose bundle identifier is "io.github.jxpeng98.canisend"
        if (count of guiProcesses) is 1 then return unix id of item 1 of guiProcesses
        delay 0.1
    end repeat
end tell
error "GUI process did not reappear uniquely" number 1
APPLESCRIPT
)"; then
  echo "macOS GUI accessibility smoke: packaged app did not relaunch" >&2
  sed -n '1,160p' "$fixture_root/restart-gui.log" >&2
  exit 1
fi

if ! osascript - "$gui_pid" <<'APPLESCRIPT'
on findNamed(parentElement, targetName)
    tell application "System Events"
        try
            if (name of parentElement as text) is targetName then return parentElement
        end try
        try
            set allElements to entire contents of parentElement
            repeat with childElement in allElements
                try
                    set candidateElement to contents of childElement
                    if (name of candidateElement as text) is targetName then return candidateElement
                end try
            end repeat
        end try
    end tell
    return missing value
end findNamed

on findNamedRole(parentElement, targetName, targetRole)
    tell application "System Events"
        try
            if (name of parentElement as text) is targetName and (value of attribute "AXRole" of parentElement as text) is targetRole then return parentElement
        end try
        try
            set allElements to entire contents of parentElement
            repeat with childElement in allElements
                try
                    set candidateElement to contents of childElement
                    if (name of candidateElement as text) is targetName and (value of attribute "AXRole" of candidateElement as text) is targetRole then return candidateElement
                end try
            end repeat
        end try
    end tell
    return missing value
end findNamedRole

on findValued(parentElement, targetValue)
    tell application "System Events"
        try
            if (value of parentElement as text) is targetValue then return parentElement
        end try
        try
            set allElements to entire contents of parentElement
            repeat with childElement in allElements
                try
                    set candidateElement to contents of childElement
                    if (value of candidateElement as text) is targetValue then return candidateElement
                end try
            end repeat
        end try
    end tell
    return missing value
end findValued

on run arguments
    set guiPid to item 1 of arguments as integer
    tell application "System Events"
        set guiProcess to missing value
        repeat 300 times
            if exists (first process whose unix id is guiPid) then
                set guiProcess to first process whose unix id is guiPid
                if (count of windows of guiProcess) > 0 then exit repeat
            end if
            delay 0.1
        end repeat
        if guiProcess is missing value then error "GUI process did not relaunch" number 1
        tell guiProcess
            set frontmost to true
            set appWindow to window 1
            set restoredSettings to missing value
            repeat 40 times
                set restoredSettings to my findValued(appWindow, "设置与诊断")
                if restoredSettings is not missing value then exit repeat
                delay 0.1
            end repeat
            if restoredSettings is missing value then error "Chinese Settings route did not restore" number 1

            set switchToEnglish to my findNamed(appWindow, "English")
            if switchToEnglish is missing value then error "English locale switch missing after restart" number 1
            click switchToEnglish
            delay 0.4

            set agentControl to my findNamedRole(appWindow, "Agent integration", "AXButton")
            if agentControl is missing value then error "Agent navigation missing after restart" number 1
            click agentControl
            delay 0.5

            set inAppControl to missing value
            repeat 40 times
                set inAppControl to my findNamed(appWindow, "In-App read-only")
                if inAppControl is not missing value then exit repeat
                delay 0.1
            end repeat
            if inAppControl is missing value then error "in-App Agent bridge missing after restart" number 1
            click inAppControl
            delay 0.4

            set agentMessage to my findNamedRole(appWindow, "Conversation", "AXTextArea")
            if agentMessage is missing value then error "Agent message control missing after restart" number 1
            click agentMessage
            set value of agentMessage to "Resume the local session fixture."
            set providerConsent to missing value
            repeat 40 times
                set providerConsent to my findNamedRole(appWindow, "I confirm this local runtime may read the selected workspace and send necessary context to its configured provider. The host stores its own transcript.", "AXCheckBox")
                if providerConsent is not missing value then exit repeat
                delay 0.1
            end repeat
            if providerConsent is missing value then error "Agent provider consent missing after restart" number 1
            if (value of providerConsent as boolean) is false then click providerConsent

            set sendControl to missing value
            repeat 40 times
                set sendControl to my findNamedRole(appWindow, "Send message", "AXButton")
                if sendControl is not missing value and (value of attribute "AXEnabled" of sendControl as boolean) is true then exit repeat
                delay 0.1
            end repeat
            if sendControl is missing value or (value of attribute "AXEnabled" of sendControl as boolean) is false then error "Agent resume action unavailable after restart" number 1
            click sendControl
        end tell

        set resumedResponse to missing value
        repeat 40 times
            delay 0.1
            tell guiProcess
                set resumedResponse to my findValued(appWindow, "Fixture resumed turn completed.")
            end tell
            if resumedResponse is not missing value then exit repeat
        end repeat
        if resumedResponse is missing value then error "Agent session did not resume after restart" number 1
        tell guiProcess
            set profileControl to my findNamedRole(appWindow, "Profile", "AXButton")
            if profileControl is missing value then error "Profile navigation control missing after restart" number 1
            click profileControl
            delay 0.5
            set profileConsent to my findNamedRole(appWindow, "I confirm CanISend may store this reviewed profile text in the active local workspace.", "AXCheckBox")
            if profileConsent is missing value then error "profile initialization consent control missing" number 1
            if (value of profileConsent as boolean) is false then click profileConsent
            if (value of profileConsent as boolean) is false then error "profile initialization consent did not enable" number 1
            set initializeProfileControl to my findNamedRole(appWindow, "Initialize profile", "AXButton")
            if initializeProfileControl is missing value then error "profile initialization action missing" number 1
            if (value of attribute "AXEnabled" of initializeProfileControl as boolean) is false then error "profile initialization action is disabled" number 1
            click initializeProfileControl
            log "accessibility smoke: profile initialization submitted; persistence is verified after automation"
            delay 1
            keystroke "q" using command down
            log "accessibility smoke: packaged route, locale, and Agent session restart passed"
        end tell
    end tell
end run
APPLESCRIPT
then
  echo "macOS GUI accessibility smoke: restart route automation failed" >&2
  sed -n '1,160p' "$fixture_root/restart-gui.log" >&2
  exit 1
fi
wait "$launcher_pid"
gui_pid=""
launcher_pid=""

session_registry="$fixture_root/home/Library/Application Support/CanISend/agent-sessions.json"
if [[ ! -f "$session_registry" || -L "$session_registry" ]] \
  || ! jq -e \
    --arg workspace "$workspace" \
    '
      .format == "canisend.agent-session-registry/v1"
      and (.entries | length) == 1
      and .entries[0].workspace == $workspace
      and .entries[0].runtime == "codex"
      and .entries[0].job_id == null
      and .entries[0].external_session_id == "fixture-session-1"
    ' "$session_registry" >/dev/null; then
  echo "macOS GUI accessibility smoke: Agent session registry did not retain the exact scope" >&2
  exit 1
fi
runtime_invocations="$fixture_root/home/.canisend-fake-codex-invocations"
if [[ ! -f "$runtime_invocations" || -L "$runtime_invocations" ]] \
  || [[ "$(wc -l < "$runtime_invocations" | tr -d ' ')" != "2" ]] \
  || ! grep -Fqx \
    'exec --json --sandbox read-only --skip-git-repo-check -' \
    "$runtime_invocations" \
  || ! grep -Fqx \
    'exec --sandbox read-only resume --json --skip-git-repo-check fixture-session-1 -' \
    "$runtime_invocations"; then
  echo "macOS GUI accessibility smoke: Agent runtime did not create then resume the exact session" >&2
  exit 1
fi
if [[ -e "$fixture_root/home/.codex" || -L "$fixture_root/home/.codex" ]]; then
  echo "macOS GUI accessibility smoke: test escaped the isolated fake Codex runtime" >&2
  exit 1
fi
installed_cli="$fixture_root/home/.local/bin/canisend"
if [[ ! -f "$installed_cli" || -L "$installed_cli" ]]; then
  echo "macOS GUI accessibility smoke: GUI did not install a regular CLI executable" >&2
  exit 1
fi
if ! cmp -s "$host" "$installed_cli"; then
  echo "macOS GUI accessibility smoke: installed CLI differs from the unified desktop host" >&2
  exit 1
fi
if ! "$installed_cli" version --json | jq -e '.ok == true' >/dev/null; then
  echo "macOS GUI accessibility smoke: installed unified CLI version command failed" >&2
  exit 1
fi
if ! "$installed_cli" --workspace "$workspace" doctor --json | jq -e '.ok == true' >/dev/null; then
  echo "macOS GUI accessibility smoke: installed unified CLI doctor command failed" >&2
  exit 1
fi
cli_help="$({ "$installed_cli" 2>&1 || true; })"
if ! printf '%s\n' "$cli_help" | grep -Fq 'Usage: canisend'; then
  echo "macOS GUI accessibility smoke: installed unified CLI did not default to CLI help" >&2
  exit 1
fi
if [[ ! -f "$fixture_root/home/.zprofile" || -L "$fixture_root/home/.zprofile" ]]; then
  echo "macOS GUI accessibility smoke: PATH action did not create a regular .zprofile" >&2
  exit 1
fi
if ! grep -Fqx '# >>> CanISend CLI PATH >>>' "$fixture_root/home/.zprofile" \
  || ! grep -Fqx '# <<< CanISend CLI PATH <<<' "$fixture_root/home/.zprofile"; then
  echo "macOS GUI accessibility smoke: PATH action did not create the managed profile block" >&2
  exit 1
fi
profile_json="$("$host" --workspace "$workspace" profile source list --json)"
if ! printf '%s' "$profile_json" \
  | jq -e '.ok == true and .data.profile_revision == 1 and (.data.sources | length) == 1' \
    >/dev/null; then
  echo "macOS GUI accessibility smoke: profile initialization did not persist one revisioned source" >&2
  printf '%s\n' "$profile_json" >&2
  exit 1
fi
echo "macOS GUI accessibility smoke: Svelte landmarks, unified-host CLI install and PATH repair, profile initialization, external-first MCP permission categories, bounded runtime evidence, scoped Agent cancellation, exact session resume, route/locale restart, bilingual controls, 200% text scale, and reduced motion passed"
