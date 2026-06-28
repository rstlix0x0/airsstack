# cmux browser — Workflow Recipes

Worked patterns for common browser-automation tasks. All examples target `surface:2`; substitute
the actual surface ref. Run `cmux browser identify` to find available browser surfaces.

---

## 1. Navigate, wait, and inspect

The baseline workflow: open a URL, wait for the page to settle, then snapshot the DOM.

```sh
# Open a browser surface (uses caller workspace by default)
cmux browser open https://example.com

# Navigate in an existing surface
cmux browser surface:2 goto https://example.com

# Wait until the page is fully loaded
cmux browser surface:2 wait --load-state complete

# Snapshot the full accessibility tree (agent-readable)
cmux browser surface:2 snapshot --interactive

# Read the current URL and title to confirm navigation succeeded
cmux browser surface:2 get url
cmux browser surface:2 get title
```

If the page renders content dynamically, wait for a specific element before inspecting:

```sh
cmux browser surface:2 wait --selector "#main-content" --timeout 10
cmux browser surface:2 snapshot --interactive --selector "#main-content"
```

---

## 2. Form fill with verification

Fill a form field, submit, and verify the resulting state.

```sh
# Take a pre-action snapshot to confirm the form is present
cmux browser surface:2 snapshot --interactive

# Clear and fill the field (fill replaces; type appends)
cmux browser surface:2 fill "#username" "alice"
cmux browser surface:2 fill "#password" "hunter2"

# Submit the form
cmux browser surface:2 click "button[type=submit]" --snapshot-after

# Verify the field value before submitting (optional sanity-check)
cmux browser surface:2 get value "#username"

# After submit, confirm navigation to the expected URL
cmux browser surface:2 wait --url-contains "/dashboard" --timeout 10
cmux browser surface:2 get url
```

Using `cmux-snap` for the click collapses snapshot + act into one call:

```sh
${CLAUDE_PLUGIN_ROOT}/skills/cmux-browser/scripts/cmux-snap surface:2 click "button[type=submit]"
```

---

## 3. Debug capture — console and error log

Read browser console messages and JS errors without injecting alert() calls.

```sh
# List all console messages (log, warn, error) accumulated so far
cmux browser surface:2 console list

# List uncaught JS errors
cmux browser surface:2 errors list

# Clear logs after reviewing (to avoid re-reading stale messages next time)
cmux browser surface:2 console clear
cmux browser surface:2 errors clear

# Evaluate a JS expression and print its result
cmux browser surface:2 eval "document.querySelectorAll('form').length"
```

Combine with a `wait` to capture logs that appear after an async operation:

```sh
cmux browser surface:2 click "#run-job"
cmux browser surface:2 wait --function "window.__jobDone === true" --timeout 30
cmux browser surface:2 console list
cmux browser surface:2 errors list
```

---

## 4. Session save and restore

Persist the full browser session (cookies + storage) between automation runs.

```sh
# Save current session state to a file
cmux browser surface:2 state save /tmp/cmux-session.json

# Later — restore the saved session (loads cookies + storage, then reloads)
cmux browser surface:2 state load /tmp/cmux-session.json

# Confirm the session is active
cmux browser surface:2 get url
cmux browser surface:2 cookies get --all
```

Use `profiles` for persistent named sessions:

```sh
cmux browser surface:2 profiles add work-session
cmux browser surface:2 profiles list
```

---

## 5. Durable screenshot

Capture the current viewport to a file for later review or CI artefact storage.

```sh
# Screenshot to a timestamped file
cmux browser surface:2 screenshot --out /tmp/snap-$(date +%Y%m%d-%H%M%S).png

# Screenshot after navigation and load
cmux browser surface:2 goto https://example.com
cmux browser surface:2 wait --load-state complete
cmux browser surface:2 screenshot --out /tmp/example-home.png
```

`--out` must be an absolute path that the cmux process can write. If `--out` is omitted, the
screenshot is embedded in the terminal output (Ghostty inline images protocol).

---

## 6. Find and act on an element by role or text

Use semantic finders when CSS selectors are fragile.

```sh
# Find a button by its ARIA role and accessible name
cmux browser surface:2 find role button --name "Submit"
cmux browser surface:2 click "button[aria-label='Submit']"   # or use the CSS the find reported

# Find by visible text
cmux browser surface:2 find text "Sign in"

# Find by data-testid (recommended for automation-friendly apps)
cmux browser surface:2 find testid "login-button"
cmux browser surface:2 click "[data-testid='login-button']" --snapshot-after

# Is the element visible and enabled before clicking?
cmux browser surface:2 is visible "[data-testid='login-button']"
cmux browser surface:2 is enabled "[data-testid='login-button']"
```

---

## 7. Iframe interaction

Switch context into an iframe, act, then return to the main frame.

```sh
# Snapshot to locate the iframe selector
cmux browser surface:2 snapshot --interactive

# Switch to the iframe
cmux browser surface:2 frame selector --selector "iframe#payment-frame"

# Act inside the iframe (subsequent commands target the iframe)
cmux browser surface:2 fill "#card-number" "4111111111111111"
cmux browser surface:2 click "#pay-now" --snapshot-after

# Return to the main frame
cmux browser surface:2 frame main
cmux browser surface:2 wait --url-contains "/confirm"
```
