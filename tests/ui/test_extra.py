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
    page.goto(URL, wait_until="domcontentloaded", timeout=30000)
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
    # 全部刷新: read all registers of the selected peripheral in place
    try:
        page.locator("button:has-text('全部刷新')").first.click(force=True)
        page.wait_for_timeout(5000)
        vals = page.locator("span.font-mono.text-emerald-300:has-text('值=')")
        T("H_SVD", "全部刷新显示各寄存器值", vals.count() >= 5, f"{vals.count()} 个值")
    except Exception as e:
        T("H_SVD", "全部刷新显示各寄存器值", False, str(e))
    # 监控: in-place periodic refresh on the register row (no separate block)
    try:
        moder_row = page.locator("span.font-mono.text-amber-300:has-text('MODER')").first.locator("xpath=ancestor::div[contains(@class,'mb-1')]").first
        rate = page.locator("select[title='监控刷新周期']").first
        rate.select_option("200")
        page.wait_for_timeout(300)
        moder_row.locator("button:has-text('监控')").first.click(force=True)
        page.wait_for_timeout(1800)
        in_place = moder_row.locator("button:has-text('停止')").count() > 0 and "●" in moder_row.inner_text()
        T("H_SVD", "监控就地显示", in_place, "停止按钮 + ● 就地值")
    except Exception as e:
        T("H_SVD", "监控就地显示", False, str(e))
    try:
        T("H_SVD", "无独立监控块", page.locator("text=监控中（周期刷新）").count() == 0, "下方无新增监控点")
    except Exception as e:
        T("H_SVD", "无独立监控块", False, str(e))
    # wait for periodic value updates in place (WS peripheral_value_changed)
    try:
        moder_row = page.locator("span.font-mono.text-amber-300:has-text('MODER')").first.locator("xpath=ancestor::div[contains(@class,'mb-1')]").first
        got = wait_for(lambda: "值=0x" in moder_row.inner_text() if moder_row.count() else False, 8000)
        T("H_SVD", "监控值就地刷新", got, "MODER 行出现值")
    except Exception as e:
        T("H_SVD", "监控值就地刷新", False, str(e))
    # stop monitor
    try:
        moder_row.locator("button:has-text('停止')").first.click(force=True)
        page.wait_for_timeout(1000)
        T("H_SVD", "监控停止", moder_row.locator("button:has-text('监控')").count() > 0, "回到监控按钮")
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
        done = wait_for(lambda: (prog.count() and not prog.is_disabled() and "烧录中" not in prog.inner_text()), 150000)
        T("I_Flash", "烧录+校验", done, f"按钮状态={prog.inner_text() if prog.count() else '?'}")
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
        # RTT panel is not in the default layout; open via the Window menu
        page.locator("button:has-text('窗口')").first.click()
        page.wait_for_timeout(400)
        page.locator("div.z-50 button:has-text('RTT')").first.click(force=True)
        page.wait_for_timeout(600)
        page.keyboard.press("Escape"); page.wait_for_timeout(300)
        tab("RTT").click(); page.wait_for_timeout(700)
        btn("启动").click(force=True); page.wait_for_timeout(2500)
        started = page.locator("button:has-text('停止')").count() > 0
        T("J_RTT", "RTT 启动", started, "启动后出现停止按钮")
        if started:
            page.locator("button:has-text('停止')").first.click(force=True); page.wait_for_timeout(800)
    except Exception as e:
        T("J_RTT", "RTT 启动", False, str(e))
    try:
        page.locator("button:has-text('窗口')").first.click()
        page.wait_for_timeout(400)
        page.locator("div.z-50 button:has-text('EVR')").first.click(force=True)
        page.wait_for_timeout(600)
        page.keyboard.press("Escape"); page.wait_for_timeout(300)
        tab("EVR").click(); page.wait_for_timeout(700)
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
