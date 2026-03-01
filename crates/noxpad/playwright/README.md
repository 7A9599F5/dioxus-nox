# Noxpad Playwright Specs

## Install

```bash
cd crates/noxpad/playwright
npm install
npm run install:browsers
```

## Run

```bash
npm test
```

By default, the Playwright config will start `noxpad` using:

```bash
dx serve -p noxpad --web --port 44253 --open false --interactive false --watch false --hot-reload false
```

If you already have a dev server running and want to reuse it:

```bash
NOXPAD_MANAGED_SERVER=0 NOXPAD_BASE_URL=http://127.0.0.1:44253 npm test
```
