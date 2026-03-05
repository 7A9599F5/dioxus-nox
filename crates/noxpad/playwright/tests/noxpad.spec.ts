import { expect, test } from '@playwright/test';

function attachErrorCollectors(page: import('@playwright/test').Page) {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];

  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  page.on('pageerror', (err) => {
    pageErrors.push(String(err));
  });

  return { consoleErrors, pageErrors };
}

async function placeCaretByTextOffset(
  page: import('@playwright/test').Page,
  selector: string,
  needle: string,
  localOffset: number,
) {
  await page.evaluate(
    ({ selector, needle, localOffset }) => {
      const root = document.querySelector(selector) as HTMLElement | null;
      if (!root) {
        return;
      }
      const text = root.innerText ?? '';
      const base = text.indexOf(needle);
      if (base < 0) {
        return;
      }
      const target = base + localOffset;

      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      let remaining = target;
      let node: Node | null = null;
      while ((node = walker.nextNode())) {
        const len = (node.nodeValue ?? '').length;
        if (remaining <= len) {
          const range = document.createRange();
          range.setStart(node, remaining);
          range.collapse(true);
          const sel = window.getSelection();
          sel?.removeAllRanges();
          sel?.addRange(range);
          root.focus();
          return;
        }
        remaining -= len;
      }
    },
    { selector, needle, localOffset },
  );
}

async function readCaretOffset(
  page: import('@playwright/test').Page,
  selector: string,
) {
  return page.evaluate(
    ({ selector }) => {
      const el = document.querySelector(selector) as HTMLElement | null;
      if (!el) {
        return 0;
      }
      const sel = window.getSelection();
      if (!sel || sel.rangeCount === 0) {
        return 0;
      }
      const range = sel.getRangeAt(0);
      const pre = range.cloneRange();
      pre.selectNodeContents(el);
      pre.setEnd(range.endContainer, range.endOffset);
      return pre.toString().length;
    },
    { selector },
  );
}

test('loads without DragContext panic', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await expect(page.getByText('Folders')).toBeVisible();
  await page.waitForTimeout(1500);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('inline active editor fills block width', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Inline' }).click();

  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();
  await paragraph.click();

  // Plain paragraphs now use TokenAwareBlockEditor (contenteditable div).
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();

  const metrics = await page.evaluate(() => {
    const editor = document.querySelector(
      '[data-md-token-editor="true"]',
    ) as HTMLElement | null;
    if (!editor || !editor.parentElement) {
      return null;
    }

    const parentRect = editor.parentElement.getBoundingClientRect();
    const editorRect = editor.getBoundingClientRect();

    return {
      parentWidth: parentRect.width,
      editorWidth: editorRect.width,
      widthRatio:
        parentRect.width > 0 ? editorRect.width / parentRect.width : 0,
    };
  });

  expect(metrics).not.toBeNull();
  expect(metrics!.parentWidth).toBeGreaterThan(100);
  expect(metrics!.widthRatio).toBeGreaterThan(0.9);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('mixed strong marker is revealed only when caret enters token context', async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const editor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(editor).toBeVisible();
  await editor.fill(
    'Borrowing lets you ref**er**ence data without taking ownership.',
  );

  await page.getByRole('button', { name: 'Inline' }).click();
  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();

  // Plain-text caret in mixed block should NOT reveal strong delimiters.
  await paragraph.click({ position: { x: 30, y: 12 } });
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();
  await expect(page.locator('[data-md-marker="inline"]')).toHaveCount(0);

  // Clicking the strong token should reveal only that token markers.
  await paragraph.locator('strong').first().click();
  const inlineMarkers = page.locator('[data-md-marker="inline"]');
  await expect(inlineMarkers).toHaveCount(2);
  await expect(inlineMarkers.first()).toHaveText('**');

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('inline list marker reveal is scoped and navigation stays stable', async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Inline' }).click();

  const listItems = page.locator('[data-md-inline-editor="true"] li');
  await expect(listItems).toHaveCount(4);

  // Clicking list content should activate token editor without raw '-' marker.
  await listItems.nth(2).click();
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();
  await expect(page.locator('[data-md-marker="block-prefix"]')).toHaveCount(0);

  // Clicking near marker zone should reveal '-' marker.
  await listItems.nth(2).click({ position: { x: 3, y: 8 } });
  const blockMarkers = page.locator('[data-md-marker="block-prefix"]');
  await expect(blockMarkers).toHaveCount(1);
  await expect(blockMarkers.first()).toContainText('-');

  // Arrow navigation should keep active token editor and visible caret context.
  await tokenEditor.press('ArrowDown');
  await expect(page.locator('[data-md-token-editor="true"]')).toHaveCount(1);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('single-line list and heading blocks do not trap caret or add phantom blank lines', async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const editor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(editor).toBeVisible();
  await editor.fill('# Top\n\n- Each value has a single owner\n\n## Next');

  await page.getByRole('button', { name: 'Inline' }).click();

  const listItem = page.locator('[data-md-inline-editor="true"] li').first();
  await expect(listItem).toBeVisible();
  await listItem.click({ position: { x: 60, y: 8 } });

  const listEditor = page.locator('li [data-md-token-editor="true"]').first();
  await expect(listEditor).toBeVisible();
  const listText = await listEditor.innerText();
  expect(listText.includes('\n')).toBeFalsy();
  expect(listText.trim()).toBe('Each value has a single owner');

  await listEditor.press('ArrowDown');
  const headingEditor = page.locator('h2 [data-md-token-editor="true"]').first();
  await expect(headingEditor).toBeVisible();
  await expect(headingEditor).toContainText('Next');

  await headingEditor.press('ArrowUp');
  await expect(listEditor).toBeVisible();

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('multi-token same-line strong editing does not jump caret', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill(
    'Borrowing lets you ref**er**ence data without taking ownership.',
  );

  await page.getByRole('button', { name: 'Inline' }).click();
  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();
  await paragraph.click({ position: { x: 120, y: 12 } });
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();

  // Repro by wrapping existing "er" in ownership via marker open/move/close.
  await placeCaretByTextOffset(
    page,
    '[data-md-token-editor="true"]',
    'ownership',
    3,
  );
  await page.keyboard.type('**');
  await tokenEditor.press('ArrowRight');
  await tokenEditor.press('ArrowRight');
  await page.keyboard.type('**');
  const rendered = await tokenEditor.innerText();
  expect(rendered).toContain('own**er**ship.');
  const base = rendered.indexOf('own**er**ship');
  expect(base).toBeGreaterThanOrEqual(0);
  const caretOffset = await readCaretOffset(page, '[data-md-token-editor="true"]');
  expect(caretOffset).toBe(base + 9);

  await page.getByRole('button', { name: 'Source' }).click();
  await expect(
    page.getByText(
      'Borrowing lets you ref**er**ence data without taking own**er**ship.',
    ),
  ).toBeVisible();

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('rapid delimiter typing stays local to intended word', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill(
    'Borrowing lets you ref**er**ence data without taking ownership.',
  );

  await page.getByRole('button', { name: 'Inline' }).click();
  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();
  await paragraph.click({ position: { x: 120, y: 12 } });
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();

  await placeCaretByTextOffset(page, '[data-md-token-editor="true"]', 'ownership', 3);
  await page.keyboard.type('**');
  await tokenEditor.press('ArrowRight');
  await tokenEditor.press('ArrowRight');
  await page.keyboard.type('**');
  const rendered = await tokenEditor.innerText();
  expect(rendered).toContain('own**er**ship.');
  const ownershipIdx = rendered.indexOf('own**er**ship');
  const caretOffset = await readCaretOffset(page, '[data-md-token-editor="true"]');
  expect(caretOffset).toBe(ownershipIdx + 9);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('caret does not step backward when markers conceal after ArrowRight', async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill(
    'Borrowing lets you reference data without taking own**er**ship.',
  );

  await page.getByRole('button', { name: 'Inline' }).click();
  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();
  await paragraph.click({ position: { x: 120, y: 12 } });
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();

  await paragraph.locator('strong').first().click();
  await expect(page.locator('[data-md-marker="inline"]')).toHaveCount(2);

  // Position caret after closing ** in own**er**ship, then move right once.
  await placeCaretByTextOffset(
    page,
    '[data-md-token-editor="true"]',
    'own**er**ship',
    9,
  );
  await tokenEditor.press('ArrowRight');
  await page.waitForTimeout(40);

  const rendered = await tokenEditor.innerText();
  expect(rendered).toContain('ownership.');
  expect(rendered).not.toContain('**');
  const base = rendered.indexOf('ownership');
  const caretOffset = await readCaretOffset(page, '[data-md-token-editor="true"]');

  // Expected after one ArrowRight: own[er]s#hip => position after 's'
  expect(caretOffset).toBe(base + 6);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('typing second closing star keeps caret after ** marker', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill(
    'Borrowing lets you ref**er**ence data without taking own**er*ship.',
  );

  await page.getByRole('button', { name: 'Inline' }).click();
  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();
  await paragraph.click({ position: { x: 120, y: 12 } });
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();

  await placeCaretByTextOffset(
    page,
    '[data-md-token-editor="true"]',
    'own**er*ship',
    8,
  );
  await page.keyboard.type('*');

  const rendered = await tokenEditor.innerText();
  expect(rendered).toContain('own**er**ship.');
  const base = rendered.indexOf('own**er**ship');
  expect(base).toBeGreaterThanOrEqual(0);
  const caretOffset = await readCaretOffset(page, '[data-md-token-editor="true"]');
  // own**er**#ship
  expect(caretOffset).toBe(base + 9);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('closing ** via mouseup-then-type does not land caret inside delimiter', async ({
  page,
}) => {
  // Regression test for the async restore-generation race:
  // onmouseup queues Spawn A (caret restore); oninput bumps restore_generation
  // so Spawn A self-cancels instead of overriding the correct caret position.
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill(
    'Borrowing lets you ref**er**ence data without taking own**er*ship.',
  );

  await page.getByRole('button', { name: 'Inline' }).click();
  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();
  await paragraph.click({ position: { x: 120, y: 12 } });
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();

  // Set cursor to position 8 within 'own**er*ship' via DOM API (no onmouseup).
  await placeCaretByTextOffset(
    page,
    '[data-md-token-editor="true"]',
    'own**er*ship',
    8,
  );

  // Dispatch a synthetic mouseup to simulate the browser event after a real
  // click. Dioxus onmouseup reads the cursor (pos 8) and queues a caret-restore
  // spawn (Spawn A). We type immediately without waiting so that oninput races
  // against Spawn A — the restore_generation guard must cancel Spawn A.
  await page.evaluate(() => {
    document
      .querySelector('[data-md-token-editor="true"]')
      ?.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
  });
  await page.keyboard.type('*');

  const rendered = await tokenEditor.innerText();
  expect(rendered).toContain('own**er**ship.');
  const base = rendered.indexOf('own**er**ship');
  expect(base).toBeGreaterThanOrEqual(0);
  const caretOffset = await readCaretOffset(page, '[data-md-token-editor="true"]');
  // own**er**#ship — caret must land after '**', not inside it
  expect(caretOffset).toBe(base + 9);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic2 = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic2, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});

test('Backspace at start of second paragraph removes one newline from gap', async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill('First paragraph\n\nSecond paragraph');

  await page.getByRole('button', { name: 'Inline' }).click();

  const secondP = page.locator('[data-md-inline-editor="true"] p').nth(1);
  await expect(secondP).toBeVisible();
  await secondP.click({ position: { x: 2, y: 8 } });
  await page.waitForTimeout(300);

  await page.keyboard.press('Home');
  await page.waitForTimeout(300);

  await page.keyboard.press('Backspace');
  await page.waitForTimeout(500);

  await page.getByRole('button', { name: 'Source' }).click();
  const value = await sourceEditor.inputValue();
  expect(value).toBe('First paragraph\nSecond paragraph');

  const allErrors = [...consoleErrors, ...pageErrors];
  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
});

test('two Backspaces at start of second paragraph fully join paragraphs', async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill('First paragraph\n\nSecond paragraph');

  await page.getByRole('button', { name: 'Inline' }).click();

  const secondP = page.locator('[data-md-inline-editor="true"] p').nth(1);
  await expect(secondP).toBeVisible();
  await secondP.click({ position: { x: 2, y: 8 } });
  await page.waitForTimeout(300);

  await page.keyboard.press('Home');
  await page.waitForTimeout(300);

  // First Backspace: \n\n → \n
  await page.keyboard.press('Backspace');
  await page.waitForTimeout(500);

  // Second Backspace: \n removed by native browser (inside merged block)
  await page.keyboard.press('Backspace');
  await page.waitForTimeout(500);

  await page.getByRole('button', { name: 'Source' }).click();
  const value = await sourceEditor.inputValue();
  expect(value).toBe('First paragraphSecond paragraph');

  const allErrors = [...consoleErrors, ...pageErrors];
  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
});

test('Backspace at start of first block is no-op', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill('First paragraph\n\nSecond paragraph');

  await page.getByRole('button', { name: 'Inline' }).click();

  const firstP = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(firstP).toBeVisible();
  await firstP.click({ position: { x: 2, y: 8 } });
  await page.waitForTimeout(300);

  await page.keyboard.press('Home');
  await page.waitForTimeout(300);

  await page.keyboard.press('Backspace');
  await page.waitForTimeout(500);

  await page.getByRole('button', { name: 'Source' }).click();
  const value = await sourceEditor.inputValue();
  expect(value).toBe('First paragraph\n\nSecond paragraph');

  const allErrors = [...consoleErrors, ...pageErrors];
  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
});

test('Backspace in middle of text does normal deletion', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  await sourceEditor.fill('Hello World\n\nSecond line');

  await page.getByRole('button', { name: 'Inline' }).click();

  const firstP = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(firstP).toBeVisible();
  await firstP.click({ position: { x: 60, y: 8 } });
  await page.waitForTimeout(300);

  await page.keyboard.press('Backspace');
  await page.waitForTimeout(500);

  await page.getByRole('button', { name: 'Source' }).click();
  const value = await sourceEditor.inputValue();
  // One character deleted, separator intact
  expect(value).toContain('\n\n');
  expect(value.length).toBe('Hello World\n\nSecond line'.length - 1);

  const allErrors = [...consoleErrors, ...pageErrors];
  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
});

test('typing ** bold from scratch keeps caret after closing **', async ({ page }) => {
  // Regression: when NO pre-existing ** pair exists on the block, completing the
  // closing ** left the cursor between the two * chars (*#*) instead of after (**#).
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Source' }).click();

  const sourceEditor = page.locator('textarea[id^="nox-md-"][id$="-editor"]');
  await expect(sourceEditor).toBeVisible();
  // Use plain list items — no ** markup so this is the "no pre-existing pair" scenario.
  // List items always use TokenAwareBlockEditor (has_block_prefix_marker).
  await sourceEditor.fill('- reference\n- ownership\n- borrowing\n- lifetime');

  await page.getByRole('button', { name: 'Inline' }).click();
  const listItems = page.locator('[data-md-inline-editor="true"] li');
  await expect(listItems).toHaveCount(4);

  // Click the first list item to activate its token editor.
  await listItems.first().click({ position: { x: 20, y: 8 } });
  const tokenEditor = page.locator('[data-md-token-editor="true"]').first();
  await expect(tokenEditor).toBeVisible();

  // Place caret after "ref" (position 3 in "reference").
  await placeCaretByTextOffset(page, '[data-md-token-editor="true"]', 'reference', 3);
  await page.waitForTimeout(30);

  // Type the opening **.
  await page.keyboard.type('**');
  await page.waitForTimeout(50);

  // Arrow right twice to skip over "er" so caret is before "ence".
  await tokenEditor.press('ArrowRight');
  await page.waitForTimeout(30);
  await tokenEditor.press('ArrowRight');
  await page.waitForTimeout(30);

  // Type the closing ** — cursor must land AFTER both asterisks, not between them.
  await page.keyboard.type('**');
  await page.waitForTimeout(100);

  const rendered = await tokenEditor.innerText();
  // Visible text with markers shown: "ref**er**ence"
  expect(rendered).toContain('ref**er**');
  const base = rendered.indexOf('ref**er**');
  expect(base).toBeGreaterThanOrEqual(0);

  const caretOffset = await readCaretOffset(page, '[data-md-token-editor="true"]');
  // Expected: ref**er**#ence — caret at base + 9 (after the closing **)
  expect(caretOffset).toBe(base + 9);

  const allErrors = [...consoleErrors, ...pageErrors];
  const dragContextPanic3 = allErrors.find((err) =>
    err.includes('Could not find context dioxus_nox_dnd::context::DragContext'),
  );

  expect(pageErrors, `Unhandled page errors: ${pageErrors.join('\n')}`).toEqual([]);
  expect(dragContextPanic3, `Runtime errors: ${allErrors.join('\n')}`).toBeUndefined();
});
