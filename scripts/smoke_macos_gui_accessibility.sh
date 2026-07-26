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
for command in osascript; do
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
cleanup() {
  if [[ -n "$gui_pid" ]] && kill -0 "$gui_pid" 2>/dev/null; then
    kill "$gui_pid" 2>/dev/null || true
    wait "$gui_pid" 2>/dev/null || true
  fi
  rm -rf "$fixture_root"
}
trap cleanup EXIT
mkdir -p "$fixture_root/home"

HOME="$fixture_root/home" "$gui" >"$fixture_root/gui.log" 2>&1 &
gui_pid="$!"

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
            my assertCondition((count of windows) is 1, "expected one CanISend window")
            set frontmost to true
            set appWindow to window 1
            log "accessibility smoke: window ready"

            set navigationElement to my findNamed(appWindow, "Primary navigation")
            my assertCondition(navigationElement is not missing value, "navigation landmark missing")
            my assertCondition((value of attribute "AXRole" of navigationElement as text) is "AXGroup", "navigation role mismatch")

            set headingElement to my findValued(appWindow, "Accessibility & appearance")
            my assertCondition(headingElement is not missing value, "accessibility heading missing")
            my assertCondition((value of attribute "AXRole" of headingElement as text) contains "Heading", "heading role mismatch")

            set mainElement to my findNamed(appWindow, "Overview content")
            my assertCondition(mainElement is not missing value, "main content landmark missing")

            set overviewControl to my findNamed(appWindow, "Overview")
            click overviewControl
            log "accessibility smoke: semantics passed"
        end tell
        delay 0.2

        set tabAnchorFound to false
        repeat 15 times
            key code 48
            delay 0.12
            tell guiProcess
                set focusedElement to value of attribute "AXFocusedUIElement"
                if focusedElement is not missing value then
                    try
                        if (name of focusedElement as text) is "Workspace" then set tabAnchorFound to true
                    end try
                end if
            end tell
            if tabAnchorFound then exit repeat
        end repeat
        my assertCondition(tabAnchorFound, "Tab traversal could not locate the Workspace anchor")

        set expectedFocus to {"Overview", "Jobs", "Profile", "Workspaces", "Command line", "Diagnostics", "Language", "Dark appearance", "Compact density", "Reduce motion", "Text size"}
        repeat with expectedName in expectedFocus
            key code 48
            delay 0.12
            tell guiProcess
                set focusedElement to value of attribute "AXFocusedUIElement"
                my assertCondition(focusedElement is not missing value, "Tab traversal lost focus")
                set actualName to name of focusedElement as text
                my assertCondition(actualName is (expectedName as text), "unexpected Tab order: expected " & (expectedName as text) & ", got " & actualName)
            end tell
        end repeat
        log "accessibility smoke: Tab order passed"

        tell guiProcess
            set textSizeControl to value of attribute "AXFocusedUIElement"
        end tell
        repeat 10 times
            keystroke "=" using command down
            delay 0.08
        end repeat
        delay 0.5
        tell guiProcess
            my assertCondition((value of textSizeControl as text) is "200%", "200% text size did not apply")
            log "accessibility smoke: 200% text size passed"
        end tell

        key code 48 using shift down
        delay 0.5
        tell guiProcess
            set focusedElement to value of attribute "AXFocusedUIElement"
            my assertCondition((name of focusedElement as text) is "Reduce motion", "off-screen setting did not receive focus")
            set motionControl to focusedElement
            set {windowX, windowY} to position of window 1
            set {windowWidth, windowHeight} to size of window 1
            set {focusX, focusY} to position of focusedElement
            set {focusWidth, focusHeight} to size of focusedElement
            my assertCondition(focusX ≥ windowX and focusY ≥ windowY, "focused setting is above or left of the window")
            my assertCondition((focusX + focusWidth) ≤ (windowX + windowWidth), "focused setting is right of the window")
            my assertCondition((focusY + focusHeight) ≤ (windowY + windowHeight), "focused setting did not scroll into view")
            log "accessibility smoke: 200% focus visibility passed"
        end tell

        key code 49
        delay 0.3
        tell guiProcess
            my assertCondition((value of motionControl as boolean) is true, "Reduce motion did not enable")
            log "accessibility smoke: reduced motion passed"
        end tell

        keystroke "0" using command down
        delay 0.5
        tell guiProcess
            my assertCondition((value of textSizeControl as text) is "100%", "Command-0 did not restore 100% text size")
            log "accessibility smoke: 100% reset passed"
        end tell

        tell guiProcess
            set languageControl to pop up button "Language" of group 1 of appWindow
            set {languageX, languageY} to position of languageControl
            set {languageWidth, languageHeight} to size of languageControl
            click languageControl
        end tell
        delay 0.2
        tell guiProcess
            set chosenOption to click at {languageX + 20, languageY + languageHeight + 70}
        end tell
        delay 0.8
        tell guiProcess
            set languageControl to pop up button "语言" of group 1 of appWindow
            my assertCondition((value of languageControl as text) is "简体中文", "Chinese language selection did not persist in the UI")
            set appearanceControl to checkbox "深色外观" of group 1 of appWindow
            my assertCondition(appearanceControl is not missing value, "Chinese appearance control missing")
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

wait "$gui_pid"
gui_pid=""
echo "macOS GUI accessibility smoke: English and Simplified Chinese semantics, Tab order, 200% focus visibility, and reduced motion passed"
