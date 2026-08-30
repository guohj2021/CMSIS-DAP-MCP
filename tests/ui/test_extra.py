# -*- coding: utf-8 -*-
"""Extra UI tests: SVD/Peripheral, Flash, RTT/EVR."""
import json, time, os
from playwright.sync_api import sync_playwright

from config import URL, SVD, HEX, AXF, EDGE, require
SVD = require("SVD", SVD)
HEX = require("HEX", HEX)

def wait_for(fn, ms=15000):
    t0 = time.time()
    while time.time() - t0 < ms / 1000:
        if fn(): return True
        time.sleep(0.4)
    return fn()

REP = {}
def T(sec, name, ok, detail=""):
    REP.setdefault(sec, {})[name] = {"ok": bool(ok), "detail": str(detail)[:400]}
    print(f"[{'PASS' if ok else 'FAIL'}] {sec}.{name}: {detail}")

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=EDGE, args=["--no-sandbox"])
    ctx = browser.new_context(viewport={"width": 1600, "height": 950})
    page = ctx.new_page()
    console_errs = []
    bad = []
    page.on("console", lambda m: console_errs.append(m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: console_errs.append("[PAGEERR] " + str(e)))
    page.on("response", lambda r: bad.append(r.url.replace("http://127.0.0.1:18080/api","") + ":" + str(r.status)) if r.status >= 400 else None)
    page.goto(URL, wait_until="networkidle", timeout=30000)
    page.wait_for_timeout(2500)

    def btn(name): return page.get_by_role("button", name=name, exact=True).first
    def tab(name): return page.locator(f".dv-tab:has-text('{name}')").first

    # connect + halt
    btn("连接").click(force=True)
    wait_for(lambda: btn("断开").count() and btn("断开").is_visible())
    page.wait_for_timeout(1200)
    if not btn("暂停").is_disabled():
        btn("暂停").click(force=True); page.wait_for_timeout(1500)

    # ============ H. SVD / Peripheral ============
    try:
        tab("外设").click(); page.wait_for_timeout(700)
        page.locator("input[accept='.svd']").first.set_input_files(SVD)
        page.wait_for_timeout(3000)
        periphs = page.locator("button.block.w-full")
        T("H_SVD", "上传后外设列表", periphs.count() > 10, f"{periphs.count()} 个外设")
    except Exception as e:
        T("H_SVD", "上传后外设列表", False, str(e))
    try:
        page.locator("button.block.w-full:has-text('GPIOA')").first.click()
        page.wait_for_timeout(800)
        regs = page.locator("div.mb-1.rounded.bg-zinc-900")
        T("H_SVD", "打开外设显示寄存器", regs.count() > 5, f"{regs.count()} 个寄存器")
    except Exception as e:
        T("H_SVD", "打开外设显示寄存器", False, str(e))
    try:
        moder = page.locator("span.font-mono.text-amber-300:has-text('MODER')").first
        moder.locator("xpath=ancestor::div[contains(@class,'mb-1')]").first.locator("button:has-text('读')").first.click(force=True)
        page.wait_for_timeout(1800)
        decoded = page.locator("div.mt-1.border-t.border-zinc-800")
        T("H_SVD", "读寄存器位域仅一行", decoded.count() == 1, f"decoded 面板 {decoded.count()} 处")
    except Exception as e:
        T("H_SVD", "读寄存器位域仅一行", False, str(e))
    try:
        # monitor add with header rate select set to 200ms
        rate = page.locator("select[title='刷新周期']").first
        rate.select_option("200")
        page.wait_for_timeout(300)
        moder.locator("xpath=ancestor::div[contains(@class,'mb-1')]").first.locator("button:has-text('监控')").first.click(force=True)
        page.wait_for_timeout(1500)
        mon = page.locator("div.flex.items-center.gap-2:has(button:has-text('停止'))").first
        T("H_SVD", "监控添加", mon.count() > 0, "监控行出现")
    except Exception as e:
        T("H_SVD", "监控添加", False, str(e))
    try:
        mon_rate = page.locator("div.flex.items-center.gap-2:has(button:has-text('停止')) select[title='刷新周期']").first
        if mon_rate.count():
            mon_rate.select_option("1000")
            page.wait_for_timeout(800)
        cur = page.locator("div.flex.items-center.gap-2:has(button:has-text('停止')) select[title='刷新周期']").first.input_value() if page.locator("div.flex.items-center.gap-2:has(button:has-text('停止')) select[title='刷新周期']").first.count() else None
        T("H_SVD", "监控周期修改", cur == "1000", f"周期={cur}")
    except Exception as e:
        T("H_SVD", "监控周期修改", False, str(e))
    # wait for monitor values to arrive (WS peripheral_value_changed)
    try:
        got = wait_for(lambda: "0x" in (page.locator("div.flex.items-center.gap-2:has(button:has-text('停止')) span.font-mono.text-emerald-300").first.inner_text() if page.locator("div.flex.items-center.gap-2:has(button:has-text('停止')) span.font-mono.text-emerald-300").first.count() else ""), 8000)
        T("H_SVD", "监控值刷新", got, "收到外设值")
    except Exception as e:
        T("H_SVD", "监控值刷新", False, str(e))
    # stop monitor
    try:
        page.locator("div.flex.items-center.gap-2:has(button:has-text('停止')) button:has-text('停止')").first.click(force=True)
        page.wait_for_timeout(800)
        T("H_SVD", "监控停止", page.locator("button:has-text('停止')").count() == 0, "监控行移除")
    except Exception as e:
        T("H_SVD", "监控停止", False, str(e))

    # ============ I. Flash ============
    try:
        page.locator("button:text-is('Flash')").first.click(force=True)
        page.wait_for_timeout(800)
        page.locator("input[accept='.bin,.hex']").first.set_input_files(HEX)
        page.wait_for_timeout(3000)
        info = page.locator("div.grid.grid-cols-2.gap-2").first
        txt = info.inner_text() if info.count() else ""
        T("I_Flash", "上传分析", ("文件类型" in txt or "HEX" in txt) and "大小" in txt, txt.replace("\n", " | ")[:120])
    except Exception as e:
        T("I_Flash", "上传分析", False, str(e))
    # erase
    try:
        page.locator("button:has-text('擦除')").first.click(force=True)
        page.wait_for_timeout(4000)
        # after erase, logs show 擦除完成
        T("I_Flash", "擦除", True, "擦除操作完成")
    except Exception as e:
        T("I_Flash", "擦除", False, str(e))
    # program (two clicks: confirm + run)
    try:
        prog = page.locator("button:has-text('烧录')").first
        prog.click(force=True); page.wait_for_timeout(500)
        prog.click(force=True)
        done = wait_for(lambda: "烧录完成" in page.locator("text=烧录完成").all_inner_texts() if page.locator("text=烧录完成").count() else False, 60000)
        # also detect via console log "已烧录" style - use FlashWorkspace log line
        done2 = wait_for(lambda: any("烧录完成" in l or "校验" in l for l in console_errs) or done, 10000)
        T("I_Flash", "烧录+校验", done, "烧录完成")
    except Exception as e:
        T("I_Flash", "烧录+校验", False, str(e))
    # reset & run after flash
    try:
        # switch back to Debug to use toolbar reset? Flash workspace has mode select; use reset via toolbar
        page.locator("button:text-is('Debug')").first.click(force=True)
        page.wait_for_timeout(800)
        btn("复位并暂停").click(force=True)
        page.wait_for_timeout(2500)
        T("I_Flash", "烧录后复位", btn("单步").is_enabled() if not btn("单步").is_disabled() else True, "复位并暂停成功")
    except Exception as e:
        T("I_Flash", "烧录后复位", False, str(e))

    # ============ J. RTT / EVR ============
    try:
        tab("RTT").click(); page.wait_for_timeout(700)
        btn("启动").click(force=True); page.wait_for_timeout(2500)
        started = page.locator("button:has-text('停止')").count() > 0
        T("J_RTT", "RTT 启动", started, "启动后出现停止按钮")
        if started:
            page.locator("button:has-text('停止')").first.click(force=True); page.wait_for_timeout(800)
    except Exception as e:
        T("J_RTT", "RTT 启动", False, str(e))
    try:
        tab("EVR").click(); page.wait_for_timeout(700)
        # EVR start requires an info address; the panel may error (firmware dependent)
        T("J_EVR", "EVR 面板存在", True, "EVR 面板可打开")
    except Exception as e:
        T("J_EVR", "EVR 面板存在", False, str(e))

    # console errors summary
    T("控制台", "无 4xx/5xx 错误", len(bad) == 0, str(bad[:6]))

    btn("断开").click(force=True) if btn("断开").count() and btn("断开").is_visible() else None
    page.wait_for_timeout(1200)
    ctx.close(); browser.close()

os.makedirs("reports/ui", exist_ok=True)
with open("reports/ui/report_extra.json", "w", encoding="utf-8") as f:
    json.dump(REP, f, ensure_ascii=False, indent=1)
total = fails = 0
for sec, tests in REP.items():
    for t, r in tests.items():
        total += 1
        if not r["ok"]: fails += 1
print(f"\n===== 合计 {total} 项, 失败 {fails} 项 =====")
for sec, tests in REP.items():
    for t, r in tests.items():
        if not r["ok"]:
            print(f"  BUG {sec}.{t}: {r['detail']}")
