"""End-to-end keyboard drag-and-drop test for noxpad.

Exercises the dnd keyboard-drop code path that issue #45 refactored:
both the Space-arm and Enter-arm of `DragContextProvider`'s onkeydown handler
go through the same `handle_keyboard_drop` helper. This test verifies that
both keys still successfully reorder items.

Run:
    python3 -m venv .venv
    .venv/bin/pip install playwright pytest pytest-playwright
    .venv/bin/playwright install chromium
    NOXPAD_URL=http://localhost:8911 .venv/bin/pytest test_keyboard_drag.py -v

The dev server must already be running:
    dx serve -p noxpad --port 8911
"""

from __future__ import annotations

import os

import pytest
from playwright.sync_api import Page, expect, sync_playwright

NOXPAD_URL = os.environ.get("NOXPAD_URL", "http://localhost:8911")
DRAG_ITEM = "[data-dnd-id]"


@pytest.fixture(scope="session")
def browser():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=os.environ.get("HEADLESS", "1") == "1")
        yield browser
        browser.close()


@pytest.fixture
def page(browser) -> Page:
    context = browser.new_context()
    page = context.new_page()
    page.goto(NOXPAD_URL)
    # Wait for hydration: items render after WASM boots.
    page.wait_for_selector(DRAG_ITEM, timeout=15_000)
    yield page
    context.close()


def get_item_order(page: Page) -> list[str]:
    """Return the current ordered list of all draggable item IDs."""
    return page.eval_on_selector_all(
        DRAG_ITEM, "els => els.map(e => e.getAttribute('data-dnd-id'))"
    )


def keyboard_drag(page: Page, item_id: str, *, drop_key: str, steps: int = 1) -> None:
    """Pick up `item_id` with Space, ArrowDown `steps` times, drop with `drop_key`.

    `drop_key` must be either ' ' (Space) or 'Enter' — the two keys that
    `handle_keyboard_drop` services in DragContextProvider.
    """
    assert drop_key in (" ", "Enter"), "drop_key must be Space or Enter"

    item = page.locator(f'{DRAG_ITEM}[data-dnd-id="{item_id}"]').first
    item.scroll_into_view_if_needed()
    item.focus()
    expect(item).to_be_focused()

    # Pick up with Space
    page.keyboard.press(" ")
    # Move
    for _ in range(steps):
        page.keyboard.press("ArrowDown")
    # Drop
    page.keyboard.press(drop_key)
    # Allow the next animation frame for DOM commit.
    page.wait_for_timeout(150)


@pytest.mark.parametrize("drop_key,label", [(" ", "Space"), ("Enter", "Enter")])
def test_keyboard_drop_completes_without_immediate_cancel(
    page: Page, drop_key: str, label: str
) -> None:
    """Regression guard for issue #58.

    Before the fix, pressing Space on a focused `SortableItem` started a
    keyboard drag and immediately ended it within the same event tick (the
    `SortableItem`'s and `DragContextProvider`'s `onkeydown` handlers both
    fired on the bubbling Space, the second one calling `handle_keyboard_drop`
    against a freshly-grabbed drag and announcing
    "Drop cancelled, item returned to start").

    The fix moves keyboard activation onto the provider as the single owner
    of keyboard drag lifecycle, and stops `DropZone`'s onkeydown from acting
    on bubbled child events. After the fix:
      - pressing Space puts the wrapper into keyboard-drag mode
      - the announcement after Space is "Grabbed item, …", not "Drop cancelled"
      - ArrowDown advances the cursor (announces "Position N of M")
      - the drop key fires `handle_keyboard_drop` which announces "Item dropped, …"

    The DOM-order assertion is intentionally not part of this test: noxpad's
    folder reorder reactivity is a separate, pre-existing issue (verified
    against `origin/main` with pointer drag) that is out of scope for #58.
    """
    before = get_item_order(page)
    assert len(before) >= 2, f"need at least 2 draggable items, got {before}"

    target = before[0]
    item = page.locator(f'{DRAG_ITEM}[data-dnd-id="{target}"]').first
    item.scroll_into_view_if_needed()
    item.focus()
    expect(item).to_be_focused()

    live_region = page.locator('[role="status"]').first

    # Pick up with Space.
    page.keyboard.press(" ")
    page.wait_for_timeout(100)
    grab_text = live_region.text_content() or ""
    assert "Grabbed" in grab_text, (
        f"{label}: Space did not start a keyboard drag (announcement: {grab_text!r})"
    )
    assert "cancelled" not in grab_text.lower(), (
        f"{label}: Space immediately cancelled the drag (announcement: {grab_text!r}) — #58 regression"
    )
    assert page.locator("[data-keyboard-active]").count() >= 1, (
        f"{label}: [data-keyboard-active] never appeared on the wrapper"
    )

    # Advance one position; should announce a new position, not cancel.
    page.keyboard.press("ArrowDown")
    page.wait_for_timeout(100)
    move_text = live_region.text_content() or ""
    assert "Position" in move_text, (
        f"{label}: ArrowDown did not move the keyboard cursor (announcement: {move_text!r})"
    )

    # Drop with the parametrized key. Must announce "Item dropped", not cancel.
    page.keyboard.press(drop_key)
    page.wait_for_timeout(200)
    drop_text = live_region.text_content() or ""
    assert "dropped" in drop_text.lower(), (
        f"{label}: drop key did not commit the drag (announcement: {drop_text!r})"
    )
    assert "cancelled" not in drop_text.lower(), (
        f"{label}: drop key was treated as a cancel (announcement: {drop_text!r}) — #58 regression"
    )


def test_escape_cancels_keyboard_drag(page: Page) -> None:
    """Escape during a keyboard drag must restore the original order.

    Sanity-check the inverse: keyboard drag started but cancelled.
    """
    before = get_item_order(page)
    target = before[0]

    item = page.locator(f'{DRAG_ITEM}[data-dnd-id="{target}"]').first
    item.scroll_into_view_if_needed()
    item.focus()
    page.keyboard.press(" ")
    page.keyboard.press("ArrowDown")
    page.keyboard.press("Escape")
    page.wait_for_timeout(150)

    after = get_item_order(page)
    assert before == after, (
        f"Escape did not cancel drag.\n  before: {before}\n  after:  {after}"
    )
