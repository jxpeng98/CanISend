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
source "$script_dir/lib/native_paths.sh"
app="$(canisend_absolute_path "$1")"
manifest="$app.manifest.json"
gui="$app/Contents/MacOS/canisend-gui"
for command in open osascript; do
  command -v "$command" >/dev/null
done
if [[ ! -d "$app" || -L "$app" ]]; then
  echo "macOS GUI accessibility smoke: app must be a regular directory: $app" >&2
  exit 1
fi
if [[ ! -f "$gui" || -L "$gui" ]]; then
  echo "macOS GUI accessibility smoke: GUI executable is missing: $gui" >&2
  exit 1
fi

"$script_dir/verify_macos_gui_app.sh" "$app" "$manifest"

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-gui-accessibility.XXXXXX")"
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
  rm -rf "$fixture_root"
}
trap cleanup EXIT
mkdir -p "$fixture_root/home"

open -n -W \
  --env "HOME=$fixture_root/home" \
  --stdout "$fixture_root/gui.log" \
  --stderr "$fixture_root/gui.log" \
  "$app" &
launcher_pid="$!"

if ! gui_pid="$(osascript - <<'APPLESCRIPT'
tell application "System Events"
    repeat 100 times
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
            repeat with childElement in UI elements of parentElement
                set foundElement to my findNamed(childElement, targetName)
                if foundElement is not missing value then return foundElement
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
            repeat with childElement in UI elements of parentElement
                set foundElement to my findNamedRole(childElement, targetName, targetRole)
                if foundElement is not missing value then return foundElement
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
            repeat with childElement in UI elements of parentElement
                set foundElement to my findValued(childElement, targetValue)
                if foundElement is not missing value then return foundElement
            end repeat
        end try
    end tell
    return missing value
end findValued

on run arguments
    set guiPid to item 1 of arguments as integer
    tell application "System Events"
        set guiProcess to missing value
        repeat 100 times
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

            set navigationElement to my findNamed(appWindow, "Primary navigation")
            if navigationElement is missing value then
                set switchToEnglish to my findNamed(appWindow, "English")
                my assertCondition(switchToEnglish is not missing value, "neither English nor Chinese navigation was exposed")
                click switchToEnglish
                delay 0.4
                set navigationElement to my findNamed(appWindow, "Primary navigation")
            end if
            my assertCondition(navigationElement is not missing value, "navigation landmark missing")
            my assertCondition((value of attribute "AXRole" of navigationElement as text) is "AXGroup", "navigation role mismatch")

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
  exit 1
fi

wait "$launcher_pid"
gui_pid=""
launcher_pid=""
echo "macOS GUI accessibility smoke: Svelte landmarks, bilingual controls, 200% text scale, and reduced motion passed"
