"""Live Playwright validation of inline-editor fixes #87, #75c, #75e and the
#74 regression on the noxpad WEB build (http://localhost:8911).

Authoritative document source = Source-mode [data-md-editor] textarea.
Inline surfaces: div[data-md-token-editor="true"] (active block), inactive
blocks div/li[data-md-inline-block="true"]. Only the block under the cursor
renders as a token-editor; click an inactive block to activate it.
"""
import pytest
from playwright.sync_api import sync_playwright

URL = "http://localhost:8911"


# ── JS helpers injected via page.evaluate ────────────────────────────────────

SET_CARET = """
(args) => {
  const [id, idx] = args;
  const root = document.getElementById(id);
  if (!root) return 'no-root';
  let remaining = idx, target = null, off = 0;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, null);
  let n;
  while ((n = walker.nextNode())) {
    const len = n.textContent.length;
    if (remaining <= len) { target = n; off = remaining; break; }
    remaining -= len;
  }
  if (!target) {
    const all = []; const w2 = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, null);
    let m; while ((m = w2.nextNode())) all.push(m);
    if (!all.length) return 'no-text';
    target = all[all.length - 1]; off = target.textContent.length;
  }
  const range = document.createRange();
  range.setStart(target, off); range.collapse(true);
  const sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(range);
  root.focus();
  return 'ok';
}
"""

# Active token-editor id + caret visual row (rectTop) + caret visible offset.
PROBE = """
() => {
  const a = document.querySelector('div[data-md-token-editor="true"]');
  if (!a) return JSON.stringify({active: null});
  const sel = window.getSelection();
  let rectTop = null, off = null;
  const br = a.getBoundingClientRect();
  if (sel.rangeCount > 0) {
    const r = sel.getRangeAt(0).cloneRange();
    const rects = r.getClientRects();
    rectTop = rects.length ? Math.round(rects[0].top) : null;
    const pre = document.createRange();
    pre.selectNodeContents(a);
    pre.setEnd(sel.getRangeAt(0).startContainer, sel.getRangeAt(0).startOffset);
    off = pre.toString().length;
  }
  return JSON.stringify({
    active: a.id, text: a.textContent, rectTop, off,
    blockHeight: Math.round(br.height)
  });
}
"""

INLINE_TEXT = """
() => {
  const a = document.querySelector('div[data-md-token-editor="true"]');
  return a ? a.textContent : null;
}
"""


# ── Page-driving helpers ─────────────────────────────────────────────────────

def click_mode(page, label):
    for t in page.query_selector_all(".mode-tab"):
        if t.inner_text().strip() == label:
            t.click()
            return True
    raise AssertionError(f"mode tab {label!r} not found")


def set_content(page, text):
    """Replace the whole document via the authoritative Source textarea."""
    click_mode(page, "Source")
    page.wait_for_timeout(300)
    ta = page.query_selector('[data-md-editor] textarea')
    assert ta is not None, "Source [data-md-editor] textarea missing"
    ta.click()
    page.keyboard.press("Control+a")
    page.keyboard.press("Delete")
    page.wait_for_timeout(120)
    page.keyboard.type(text)
    page.wait_for_timeout(450)


def source_value(page):
    """Read the authoritative raw document from the Source textarea."""
    click_mode(page, "Source")
    page.wait_for_timeout(350)
    return page.evaluate("(el)=>el.value", page.query_selector('[data-md-editor] textarea'))


def activate_block(page, needle):
    """Make the block whose visible text contains `needle` the active token editor."""
    a = page.query_selector('div[data-md-token-editor="true"]')
    if a and needle in a.inner_text():
        return a
    for b in page.query_selector_all('[data-md-inline-block="true"]'):
        if needle in b.inner_text():
            b.click()
            page.wait_for_timeout(500)
            a = page.query_selector('div[data-md-token-editor="true"]')
            if a and needle in a.inner_text():
                return a
    return None


@pytest.fixture(scope="module")
def browser_ctx():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        yield browser
        browser.close()


@pytest.fixture()
def page(browser_ctx):
    pg = browser_ctx.new_page(viewport={"width": 1200, "height": 800})
    pg.goto(URL)
    pg.wait_for_selector(".note-item", timeout=60000)  # WASM hydration gate
    pg.query_selector_all(".note-item")[0].click()
    pg.wait_for_timeout(400)
    yield pg
    pg.close()


# ── SCENARIO #87 — deleteWordBackward must not orphan inline markers ──────────

def test_87_word_delete_no_orphan_markers(page):
    set_content(page, "some **bold** text\n")
    click_mode(page, "Inline")
    page.wait_for_timeout(700)
    act = activate_block(page, "bold")
    assert act is not None, "could not activate the bold-bearing block"
    bid = act.get_attribute("id")
    assert "<strong>bold</strong>" in page.evaluate("(el)=>el.innerHTML", act)

    # Caret after visible "bold": visible text is "some bold text" -> index 9.
    assert page.evaluate(SET_CARET, [bid, 9]) == "ok"
    page.wait_for_timeout(200)
    page.keyboard.press("Control+Backspace")  # deleteWordBackward
    page.wait_for_timeout(600)

    raw = source_value(page)
    assert "some " in raw, f"expected leading 'some ' preserved, got {raw!r}"
    assert "****" not in raw, f"orphaned '****' markers present: {raw!r}"
    assert "**" not in raw, f"any orphaned '**' markers present: {raw!r}"


# ── SCENARIO #87 MID-WORD — partial word-delete must preserve BOTH delimiters ─

def test_87_word_delete_mid_word_preserves_both_markers(page):
    # Inline block "some **bold** text". Visible text is "some bold text".
    # Caret in the MIDDLE of bold (after "bo", visible index 7). A backward
    # word-delete (Control+Backspace) targets only the partial-word span up to
    # the caret; because the bold span's content is NOT wholly emptied ("ld"
    # survives), BOTH hidden `**` delimiters must remain -> "some **ld** text".
    set_content(page, "some **bold** text\n")
    click_mode(page, "Inline")
    page.wait_for_timeout(700)
    act = activate_block(page, "bold")
    assert act is not None, "could not activate the bold-bearing block"
    bid = act.get_attribute("id")
    assert "<strong>bold</strong>" in page.evaluate("(el)=>el.innerHTML", act)

    # Visible text "some bold text" -> index 7 lands after "bo" inside bold.
    assert page.evaluate(SET_CARET, [bid, 7]) == "ok"
    page.wait_for_timeout(200)
    page.keyboard.press("Control+Backspace")  # deleteWordBackward
    page.wait_for_timeout(600)

    raw = source_value(page)
    # Both delimiters preserved around the surviving "ld": "some **ld** text".
    # The browser's deleteWordBackward target span may consume the leading
    # "some " word boundary as well; what MUST hold is that both `**` remain and
    # "ld" stays wrapped — never "some ld**" (opening eaten) and never an empty
    # "some  text" (whole span eaten).
    assert raw.count("**") == 2, f"expected exactly two '**' delimiters, got {raw!r}"
    assert "**ld**" in raw, f"surviving 'ld' must stay wrapped in '**...**', got {raw!r}"
    assert "ld**" in raw and raw.index("**") < raw.index("ld**"), (
        f"opening '**' must precede 'ld', got {raw!r}"
    )
    assert "text" in raw, f"trailing ' text' must be preserved, got {raw!r}"


# ── SCENARIO #75c — multi-line (soft line break) caret must escape, not trap ──

def test_75c_multiline_caret_escape(page):
    # A paragraph with an internal newline = soft line break -> 2 visual lines,
    # one block. Surrounded by other paragraphs to escape into.
    set_content(page, "first paragraph\n\nline one here\nline two here\n\nthird paragraph\n")
    click_mode(page, "Inline")
    page.wait_for_timeout(700)
    act = activate_block(page, "line one here")
    assert act is not None, "could not activate the multi-line block"
    multiline_id = act.get_attribute("id")
    txt = act.inner_text()
    assert "\n" in txt, f"block is not multi-line: {txt!r}"

    # Caret on the FIRST visual line -> ArrowUp must escape to a PREVIOUS block.
    assert page.evaluate(SET_CARET, [multiline_id, 3]) == "ok"
    page.wait_for_timeout(250)
    page.keyboard.press("ArrowUp")
    page.wait_for_timeout(500)
    after_up = page.query_selector('div[data-md-token-editor="true"]')
    assert after_up is not None
    assert after_up.get_attribute("id") != multiline_id, (
        "ArrowUp on first visual line did NOT escape — caret trapped in block"
    )

    # Re-activate the multi-line block; caret on the LAST visual line ->
    # ArrowDown must reach a NEXT block.
    act = activate_block(page, "line one here")
    assert act is not None
    multiline_id = act.get_attribute("id")
    txt = act.inner_text()
    assert page.evaluate(SET_CARET, [multiline_id, len(txt)]) == "ok"
    page.wait_for_timeout(250)
    page.keyboard.press("ArrowDown")
    page.wait_for_timeout(500)
    after_down = page.query_selector('div[data-md-token-editor="true"]')
    assert after_down is not None
    assert after_down.get_attribute("id") != multiline_id, (
        "ArrowDown on last visual line did NOT escape — caret trapped in block"
    )


# ── SCENARIO #75e — soft-wrapped single line: ArrowDown moves within block ────

def test_75e_softwrap_single_line_moves_within(page):
    longline = ("AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF GGGGG HHHHH IIIII JJJJJ "
                "KKKKK LLLLL MMMMM NNNNN OOOOO PPPPP QQQQQ RRRRR")
    set_content(page, f"prevblock here\n\n{longline}\n\nnextblock here\n")
    # Narrow the viewport AFTER content is set, forcing soft-wrap to >=2 rows.
    page.set_viewport_size({"width": 480, "height": 800})
    page.wait_for_timeout(300)
    click_mode(page, "Inline")
    page.wait_for_timeout(700)
    act = activate_block(page, "AAAAA")
    assert act is not None, "could not activate the long single-line block"
    block_id = act.get_attribute("id")
    txt = act.inner_text()
    assert "\n" not in txt, f"block unexpectedly multi-line: {txt!r}"

    # Confirm the line actually wraps to >=2 visual rows.
    assert page.evaluate(SET_CARET, [block_id, 0]) == "ok"
    page.wait_for_timeout(150)
    row1 = __import__("json").loads(page.evaluate(PROBE))
    assert page.evaluate(SET_CARET, [block_id, len(txt)]) == "ok"
    page.wait_for_timeout(150)
    rowlast = __import__("json").loads(page.evaluate(PROBE))
    assert rowlast["rectTop"] is not None and row1["rectTop"] is not None
    assert rowlast["rectTop"] > row1["rectTop"], (
        f"line did not soft-wrap to multiple rows: row1={row1} last={rowlast}"
    )

    # Caret on ROW 1 -> ArrowDown must move DOWN one visual row WITHIN the block.
    assert page.evaluate(SET_CARET, [block_id, 2]) == "ok"
    page.wait_for_timeout(150)
    before = __import__("json").loads(page.evaluate(PROBE))
    page.keyboard.press("ArrowDown")
    page.wait_for_timeout(500)
    after = __import__("json").loads(page.evaluate(PROBE))
    assert after["active"] == block_id, (
        f"ArrowDown on row 1 jumped to another block (#75e regression): "
        f"before={before} after={after}"
    )
    assert after["rectTop"] is not None and before["rectTop"] is not None
    assert after["rectTop"] > before["rectTop"], (
        f"ArrowDown did not advance the caret to a lower visual row: "
        f"before={before} after={after}"
    )

    # On the LAST visual row, ArrowDown must finally escape to the next block.
    assert page.evaluate(SET_CARET, [block_id, len(txt)]) == "ok"
    page.wait_for_timeout(150)
    page.keyboard.press("ArrowDown")
    page.wait_for_timeout(500)
    escaped = page.query_selector('div[data-md-token-editor="true"]')
    assert escaped is not None
    assert escaped.get_attribute("id") != block_id, (
        "ArrowDown on the last visual row did NOT escape to the next block"
    )


# ── SCENARIO #74 — regression: held Backspace shrinks asterisks monotonically ─

def test_74_held_backspace_monotonic_shrink(page):
    set_content(page, "some **bold** text\n")
    click_mode(page, "Inline")
    page.wait_for_timeout(700)
    act = activate_block(page, "bold")
    assert act is not None
    bid = act.get_attribute("id")

    # Establish caret after the closing ** and force the reveal + Rust caret sync
    # via a real ArrowLeft/ArrowRight round-trip (keyup-driven sync).
    assert page.evaluate(SET_CARET, [bid, 9]) == "ok"  # after visible "bold"
    page.wait_for_timeout(200)
    page.keyboard.press("ArrowLeft")
    page.wait_for_timeout(300)
    page.keyboard.press("ArrowRight")
    page.wait_for_timeout(300)
    revealed = page.evaluate(INLINE_TEXT)
    assert revealed is not None and "**bold**" in revealed, (
        f"markers did not reveal after caret placement: {revealed!r}"
    )

    # Held Backspace: capture asterisk count from the revealed inline text before
    # each press. The invariant (#74) is monotonic non-increase — never grows
    # (no **->***->**** growth).
    counts = []
    for _ in range(9):
        t = page.evaluate(INLINE_TEXT)
        if t is None:
            break
        counts.append(t.count("*"))
        page.keyboard.press("Backspace")
        page.wait_for_timeout(320)
    final = page.evaluate(INLINE_TEXT)
    if final is not None:
        counts.append(final.count("*"))

    increases = [(i, counts[i - 1], counts[i])
                 for i in range(1, len(counts)) if counts[i] > counts[i - 1]]
    assert not increases, (
        f"asterisk count INCREASED during held Backspace (#74 regression): "
        f"counts={counts} increases={increases}"
    )
    assert max(counts) <= 4, (
        f"asterisk count exceeded the **bold** ceiling of 4: counts={counts}"
    )
