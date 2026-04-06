# noxpad Python Playwright tests

Reusable Python E2E tests against the noxpad demo app. Parallel to the
existing TypeScript suite in `../playwright/`.

## Setup

```bash
python3 -m venv .venv
.venv/bin/pip install playwright pytest pytest-playwright
.venv/bin/playwright install chromium
```

## Run

Start noxpad in one terminal:

```bash
dx serve -p noxpad --port 8911
```

Then in another:

```bash
NOXPAD_URL=http://localhost:8911 .venv/bin/pytest -v
```

Set `HEADLESS=0` to watch the browser.

## Tests

| File | Covers | Related issue |
|---|---|---|
| `test_keyboard_drag.py` | dnd keyboard drop (Space/Enter) and Escape cancel | #45 |
