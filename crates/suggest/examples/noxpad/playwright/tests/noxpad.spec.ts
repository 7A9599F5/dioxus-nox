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

test('inline active textarea fills block width', async ({ page }) => {
  const { consoleErrors, pageErrors } = attachErrorCollectors(page);

  await page.goto('/');
  await page.getByRole('button', { name: 'Inline' }).click();

  const paragraph = page.locator('[data-md-inline-editor="true"] p').first();
  await expect(paragraph).toBeVisible();
  await paragraph.click();

  const activeTextarea = page.locator('textarea[data-md-active-block-editor="true"]');
  await expect(activeTextarea).toBeVisible();
  await expect(activeTextarea).toHaveAttribute('rows', '1');

  const metrics = await page.evaluate(() => {
    const textarea = document.querySelector(
      'textarea[data-md-active-block-editor="true"]',
    ) as HTMLTextAreaElement | null;
    if (!textarea || !textarea.parentElement) {
      return null;
    }

    const parentRect = textarea.parentElement.getBoundingClientRect();
    const textareaRect = textarea.getBoundingClientRect();

    return {
      parentWidth: parentRect.width,
      textareaWidth: textareaRect.width,
      widthRatio:
        parentRect.width > 0 ? textareaRect.width / parentRect.width : 0,
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
