# -*- coding: utf-8 -*-
"""UI functional test harness (Playwright/Edge) against the live web server."""
import json, time, traceback, os
from playwright.sync_api import sync_playwright

from config import URL, AXF, EDGE

class Rep:
    def __init__(self):
        self.results = {}   # section -> { test -> {ok, detail} }
    def sec(self, name):
        self.results.setdefault(name, {})
        self.cur = name
    def t(self, name, ok, detail=""):
        self.results[self.cur][name] = {"ok": bool(ok), "detail": str(detail)[:400]}
        flag = "PASS" if ok else "FAIL"
        print(f"[{flag}] {self.cur}.{name}: {detail}")

def wait_for(fn, ms=12000):
    import time as _t
    t0 = _t.time()
    while _t.time() - t0 < ms / 1000:
        if fn(): return True
        _t.sleep(0.4)
    return fn()

REP = Rep()

def section(name):
    def deco(fn):
        REP.sec(name)
        fn()
    return deco

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=EDGE, args=["--no-sandbox"])
    ctx = browser.new_context(viewport={"width": 1600, "height": 950})
    page = ctx.new_page()
    console_errs = []
    page.on("console", lambda m: console_errs.append(m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: console_errs.append("[PAGEERROR] " + str(e)))
    page.goto(URL, wait_until="domcontentloaded", timeout=30000)
    page.wait_for_timeout(2500)

    def btn(name, exact=True):
        return page.get_by_role("button", name=name, exact=exact).first

    def tab(name):
        return page.locator(f".dv-tab:has-text('{name}')").first

    def wait_visible(loc, ms=30000):
        loc.wait_for(state="visible", timeout=ms)

    # ============ A. 连接 ============
    REP.sec("A_连接")
    try:
        REP.t("初始未连接", btn("连接").is_visible() and btn("运行").is_disabled(), "连接可见, 运行禁用")
    except Exception as e:
        REP.t("初始未连接", False, str(e))
    try:
        btn("连接").click(force=True)
        ok = False
        for _ in range(30):
            page.wait_for_timeout(400)
            if btn("断开").count() and btn("断开").is_visible():
                ok = True; break
        REP.t("连接成功", ok, "出现断开按钮")
    except Exception as e:
        REP.t("连接成功", False, str(e))
    try:
        btn("断开").click(force=True)
        page.wait_for_timeout(1500)
        REP.t("断开成功", btn("连接").is_visible(), "回到连接按钮")
    except Exception as e:
        REP.t("断开成功", False, str(e))

    # ============ B. 执行控制 ============
    REP.sec("B_执行控制")
    def connect():
        btn("连接").click(force=True)
        for _ in range(30):
            page.wait_for_timeout(400)
            if btn("断开").count() and btn("断开").is_visible(): return True
        return False
    try:
        connect()
        page.wait_for_timeout(1500)
        REP.t("连接后状态", True, f"运行禁用={btn('运行').is_disabled()} 暂停禁用={btn('暂停').is_disabled()}")
    except Exception as e:
        REP.t("连接后状态", False, str(e))
    try:
        halt = btn("暂停")
        if not halt.is_disabled():
            halt.click(force=True); page.wait_for_timeout(2000)
        REP.t("暂停后按钮", (not btn("运行").is_disabled()) and btn("暂停").is_disabled() and (not btn("单步").is_disabled()), f"运行可用={not btn('运行').is_disabled()} 暂停禁用={btn('暂停').is_disabled()} 单步可用={not btn('单步').is_disabled()}")
    except Exception as e:
        REP.t("暂停后按钮", False, str(e))
    try:
        before = page.locator("div[draggable='true']:has(span.font-mono)").count()
        btn("单步").click(force=True); page.wait_for_timeout(1800)
        REP.t("单步执行", True, "单步后无异常")
    except Exception as e:
        REP.t("单步执行", False, str(e))
    try:
        btn("运行").click(force=True); page.wait_for_timeout(2000)
        REP.t("运行后按钮", (not btn("暂停").is_disabled()) and btn("运行").is_disabled(), f"暂停可用={not btn('暂停').is_disabled()} 运行禁用={btn('运行').is_disabled()}")
    except Exception as e:
        REP.t("运行后按钮", False, str(e))
    try:
        btn("暂停").click(force=True); page.wait_for_timeout(1500)
        btn("复位并暂停").click(force=True); page.wait_for_timeout(2000)
        REP.t("复位并暂停", btn("暂停").is_disabled() and (not btn("单步").is_disabled()), f"暂停禁用={btn('暂停').is_disabled()} 单步可用={not btn('单步').is_disabled()}")
    except Exception as e:
        REP.t("复位并暂停", False, str(e))
    try:
        btn("复位").click(force=True); page.wait_for_timeout(2000)
        REP.t("复位运行", not btn("暂停").is_disabled(), f"暂停可用={not btn('暂停').is_disabled()}")
    except Exception as e:
        REP.t("复位运行", False, str(e))

    # ============ C. 寄存器 ============
    REP.sec("C_寄存器")
    try:
        btn("暂停").click(force=True); page.wait_for_timeout(2000)
        regs = page.locator("div.flex.cursor-pointer.justify-between")
        n = regs.count()
        names = page.locator("span.text-zinc-400").all_inner_texts()
        has_pc = any(x.strip().lower() in ("pc", "r15") for x in names)
        REP.t("寄存器列表非空", n > 10, f"{n} 行, PC={has_pc}")
    except Exception as e:
        REP.t("寄存器列表非空", False, str(e))
    try:
        fmt = page.locator("select:has(option[value='dec'])").first
        fmt.select_option("dec")
        page.wait_for_timeout(300)
        fmt.select_option("hex")
        page.wait_for_timeout(300)
        REP.t("格式切换", True, "HEX/DEC 切换无异常")
    except Exception as e:
        REP.t("格式切换", False, str(e))
    try:
        page.locator("input[placeholder='过滤寄存器…']").fill("pc")
        page.wait_for_timeout(300)
        n = page.locator("div.flex.cursor-pointer.justify-between").count()
        REP.t("寄存器过滤", n >= 1 and n < 30, f"过滤后 {n} 行")
        page.locator("input[placeholder='过滤寄存器…']").fill("")
    except Exception as e:
        REP.t("寄存器过滤", False, str(e))

    # ============ D. 内存 ============
    REP.sec("D_内存")
    try:
        tab("内存").click(); page.wait_for_timeout(600)
        page.locator("input.w-32.rounded.bg-zinc-800").first.fill("0x20000000")
        page.locator("button:has-text('读取')").first.click(force=True); page.wait_for_timeout(1800)
        cells = page.locator("div.flex.items-center.gap-2.whitespace-pre input")
        REP.t("内存读取", cells.count() > 0, f"{cells.count()} 个数据单元")
    except Exception as e:
        REP.t("内存读取", False, str(e))
    try:
        sel = page.locator("select:has(option[value='u16'])").first
        def per_row():
            row = page.locator("div.flex.items-center.gap-2.whitespace-pre").first
            return row.locator("input").count() if row.count() else 0
        sel.select_option("u16"); page.wait_for_timeout(400); u16n = per_row()
        sel.select_option("u8"); page.wait_for_timeout(400); u8n = per_row()
        sel.select_option("u32"); page.wait_for_timeout(400); u32n = per_row()
        REP.t("宽度切换生效", u16n == 8 and u8n == 16 and u32n == 4, f"u32={u32n}/行 u16={u16n}/行 u8={u8n}/行")
    except Exception as e:
        REP.t("宽度切换生效", False, str(e))
    try:
        page.locator("button:has-text('暂停')").click(force=True) if not page.locator("button:has-text('暂停')").first.is_disabled() else None
        page.wait_for_timeout(800)
        sel = page.locator("select:has(option[value='u16'])").first
        sel.select_option("u8")
        page.locator("input.w-32.rounded.bg-zinc-800").first.fill("0x20000000")
        page.locator("button:has-text('读取')").first.click(force=True); page.wait_for_timeout(1800)
        cell = page.locator("div.flex.items-center.gap-2.whitespace-pre input").first
        old = cell.input_value()
        newv = "FF" if old != "FF" else "00"
        cell.fill(newv); cell.press("Tab"); page.wait_for_timeout(2000)
        page.locator("button:has-text('读取')").first.click(force=True); page.wait_for_timeout(1800)
        now = page.locator("div.flex.items-center.gap-2.whitespace-pre input").first.input_value()
        REP.t("内存写后重读", now.upper() == newv, f"{old} -> {newv} -> 读回 {now}")
    except Exception as e:
        REP.t("内存写后重读", False, str(e))

    # ============ E. 断点 ============
    REP.sec("E_断点")
    try:
        # 断点 panel is not in the default layout; open via the Window menu
        page.locator("button:has-text('窗口')").first.click()
        page.wait_for_timeout(400)
        page.locator("div.z-50 button:has-text('断点')").first.click(force=True)
        page.wait_for_timeout(600)
        page.keyboard.press("Escape"); page.wait_for_timeout(300)
        tab("断点").click(); page.wait_for_timeout(600)
        page.locator("button:has-text('暂停')").click(force=True) if not page.locator("button:has-text('暂停')").first.is_disabled() else None
        page.wait_for_timeout(800)
        # set HW breakpoint at main symbol address if loaded, else 0x08000000
        page.locator(".dv-groupview:has(.dv-tab:has-text('断点')) input.w-28").first.fill("0x08000000")
        page.locator(".dv-groupview:has(.dv-tab:has-text('断点')) button:has-text('添加')").first.click(force=True); page.wait_for_timeout(1500)
        bps = page.locator("span.font-mono.text-amber-300")
        REP.t("添加断点", bps.count() > 0, f"{bps.count()} 个断点")
    except Exception as e:
        REP.t("添加断点", False, str(e))
    try:
        page.locator("button:has-text('清除')").first.click(force=True); page.wait_for_timeout(1200)
        REP.t("清除断点", True, "清除无异常")
    except Exception as e:
        REP.t("清除断点", False, str(e))

    # ============ F. 符号 ============
    REP.sec("F_符号")
    try:
        tab("符号").click(); page.wait_for_timeout(600)
        page.locator("input[accept='.elf,.axf']").first.set_input_files(AXF)
        def rows_ok():
            return page.locator("div[draggable='true']:has(span.font-mono)").count() > 0
        ok = wait_for(rows_ok, 12000)
        n = page.locator("div[draggable='true']:has(span.font-mono)").count()
        REP.t("符号行渲染", ok, f"{n} 行")
    except Exception as e:
        REP.t("符号行渲染", False, str(e))
    try:
        page.locator("input[placeholder='搜索…']").first.fill("main")
        page.wait_for_timeout(1200)
        n = page.locator("div[draggable='true']:has(span.font-mono)").count()
        page.locator("input[placeholder='搜索…']").first.fill("")
        page.wait_for_timeout(800)
        REP.t("符号搜索", n >= 1, f"搜索 main -> {n} 行")
    except Exception as e:
        REP.t("符号搜索", False, str(e))
    try:
        page.locator("button:has-text('＋Watch')").first.click(force=True)
        page.wait_for_timeout(1500)
        # Watch is a tab (right-bottom with Memory); open it to see the item
        page.locator(".dv-tab:has-text('Watch')").first.click(force=True)
        page.wait_for_timeout(1000)
        REP.t("＋Watch 按钮", page.locator("text=符号 #").count() >= 1, f"Watch 项 {page.locator('text=符号 #').count()}")
    except Exception as e:
        REP.t("＋Watch 按钮", False, str(e))

    # ============ G. Watch ============
    REP.sec("G_Watch")
    try:
        tab("Watch").click(); page.wait_for_timeout(600)
        n0 = page.locator("text=符号 #").count()
        rate = page.locator("select[title='刷新周期 (Live Watch)']").first
        if rate.count():
            rate.select_option("1000")
            page.wait_for_timeout(500)
        REP.t("Watch 项存在/周期切换", n0 >= 1, f"{n0} 项, 周期切换完成")
    except Exception as e:
        REP.t("Watch 项存在/周期切换", False, str(e))
    try:
        page.locator("button:has-text('删除')").first.click(force=True)
        page.wait_for_timeout(1200)
        n1 = page.locator("text=符号 #").count()
        REP.t("Watch 删除", n1 == n0 - 1, f"{n0} -> {n1}")
    except Exception as e:
        REP.t("Watch 删除", False, str(e))

    # ============ K. 布局/菜单/配置 ============
    REP.sec("K_布局菜单配置")
    try:
        page.locator("button:has-text('布局')").first.click()
        page.wait_for_timeout(400)
        page.locator("div.z-50 button:has-text('Quick Debug')").first.click(force=True)
        page.wait_for_timeout(800)
        tabs_n = page.locator(".dv-tab").count()
        page.locator("button:has-text('布局')").first.click()
        page.wait_for_timeout(400)
        page.locator("div.z-50 button:has-text('Full Debug')").first.click(force=True)
        page.wait_for_timeout(800)
        REP.t("布局切换", tabs_n == 4, f"Quick 4 页签 -> Full")
    except Exception as e:
        REP.t("布局切换", False, str(e))
    try:
        page.locator("button:has-text('窗口')").first.click()
        page.wait_for_timeout(400)
        items = page.locator("div.z-50 button").count()
        page.keyboard.press("Escape"); page.wait_for_timeout(300)
        REP.t("窗口菜单", items >= 10, f"{items} 个窗口项")
    except Exception as e:
        REP.t("窗口菜单", False, str(e))
    try:
        page.locator("button:has-text('配置')").first.click()
        page.wait_for_timeout(400)
        page.locator("button:has-text('保存当前布局')").first.click(force=True)
        page.wait_for_timeout(300)
        page.locator("div.z-50 input").first.fill("回归配置")
        page.locator("div.z-50 button:text-is('保存')").first.click(force=True)
        page.wait_for_timeout(600)
        page.locator("button:has-text('配置')").first.click()
        page.wait_for_timeout(400)
        has = page.locator("div.z-50 button:has-text('回归配置')").count() > 0
        page.locator("div.z-50 button[title='删除配置']").first.click(force=True)
        page.wait_for_timeout(400)
        page.keyboard.press("Escape")
        REP.t("配置保存/删除", has, f"配置出现={has}")
    except Exception as e:
        REP.t("配置保存/删除", False, str(e))

    # cleanup
    REP.sec("A_连接")
    try:
        if btn("断开").count() and btn("断开").is_visible():
            btn("断开").click(force=True)
        page.wait_for_timeout(1000)
        REP.t("结束断开", btn("连接").is_visible(), "")
    except Exception:
        pass

    REP.sec("控制台错误")
    # A single 409 on session loss is expected (status poll races the sync);
    # the poller stops after the first not_connected. 5xx are real bugs.
    bad = [e for e in console_errs if "400" not in e and "404" not in e and "409" not in e]
    REP.t("无 5xx 控制台错误", len(bad) == 0, str(console_errs[:5]))

    ctx.close(); browser.close()

# write report
os.makedirs("reports/ui", exist_ok=True)
with open("reports/ui/report_core.json", "w", encoding="utf-8") as f:
    json.dump(REP.results, f, ensure_ascii=False, indent=1)

# summary
total = fails = 0
for sec, tests in REP.results.items():
    for t, r in tests.items():
        total += 1
        if not r["ok"]: fails += 1
print(f"\n===== 合计 {total} 项, 失败 {fails} 项 =====")
for sec, tests in REP.results.items():
    for t, r in tests.items():
        if not r["ok"]:
            print(f"  BUG {sec}.{t}: {r['detail']}")
