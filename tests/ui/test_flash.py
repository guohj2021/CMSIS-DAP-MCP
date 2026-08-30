# -*- coding: utf-8 -*-
"""Flash UI test with proper completion detection + firmware verify."""
import json, time
from playwright.sync_api import sync_playwright

from config import URL, HEX, EDGE, require
HEX = require("HEX", HEX)

def wait_for(fn, ms=120000):
    t0 = time.time()
    while time.time() - t0 < ms / 1000:
        if fn(): return True
        time.sleep(0.5)
    return fn()

res = {}
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=EDGE, args=["--no-sandbox"])
    ctx = browser.new_context(viewport={"width": 1600, "height": 950})
    page = ctx.new_page()
    console_errs = []
    page.on("console", lambda m: console_errs.append(m.text) if m.type == "error" else None)
    page.goto(URL, wait_until="networkidle", timeout=30000)
    page.wait_for_timeout(2500)

    def btn(name): return page.get_by_role("button", name=name, exact=True).first

    btn("连接").click(force=True)
    wait_for(lambda: btn("断开").count() and btn("断开").is_visible(), 15000)
    page.wait_for_timeout(1200)
    if not btn("暂停").is_disabled():
        btn("暂停").click(force=True); page.wait_for_timeout(1200)

    page.locator("button:text-is('Flash')").first.click(force=True)
    page.wait_for_timeout(800)
    page.locator("input[accept='.bin,.hex']").first.set_input_files(HEX)
    page.wait_for_timeout(3000)
    info = page.locator("div.grid.grid-cols-2.gap-2").first
    res["analyze"] = info.inner_text().replace("\n", " | ")[:150] if info.count() else "NO INFO"

    # erase
    page.locator("button:has-text('擦除')").first.click(force=True)
    page.wait_for_timeout(5000)
    # program: confirm click -> run click; wait until button enabled again
    prog = page.locator("button:has-text('烧录')").first
    prog.click(force=True); page.wait_for_timeout(500)
    prog.click(force=True)
    done = wait_for(lambda: (prog.count() and not prog.is_disabled() and "烧录中" not in prog.inner_text()), 150000)
    res["program_done"] = done
    res["btn_text"] = prog.inner_text() if prog.count() else "?"

    # verify firmware: read flash 0x08000000 first bytes via memory API through UI (Debug -> memory)
    page.locator("button:text-is('Debug')").first.click(force=True)
    page.wait_for_timeout(800)
    # read via memory panel
    page.locator(".dv-tab:has-text('内存')").first.click()
    page.wait_for_timeout(600)
    page.locator("input.w-32.rounded.bg-zinc-800").first.fill("0x08000000")
    page.locator("button:has-text('读取')").first.click(force=True)
    page.wait_for_timeout(2000)
    cells = page.locator("input.w-7.bg-transparent.text-center")
    first_bytes = [cells.nth(i).input_value() for i in range(min(8, cells.count()))]
    res["flash_first_bytes"] = first_bytes
    res["flash_programmed"] = any(b != "FF" for b in first_bytes)

    res["console_errs"] = console_errs[:6]
    btn("断开").click(force=True) if btn("断开").count() and btn("断开").is_visible() else None
    page.wait_for_timeout(1200)
    ctx.close(); browser.close()

print(json.dumps(res, ensure_ascii=True, indent=1))
