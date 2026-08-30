# -*- coding: utf-8 -*-
"""Shared config for UI functional tests (Playwright/Edge).

Override via environment variables:
  UI_URL    - web server base URL (default http://127.0.0.1:18080/)
  UI_AXF    - ELF/AXF file for symbol tests
  UI_SVD    - SVD file for peripheral tests
  UI_HEX    - HEX/BIN file for flash tests
  UI_EDGE   - path to msedge.exe (auto-detected if unset)
"""
import os

URL = os.environ.get("UI_URL", "http://127.0.0.1:18080/")

def _default(name, *cands):
    v = os.environ.get(name)
    if v:
        return v
    for c in cands:
        if os.path.exists(c):
            return c
    return None

AXF = _default("UI_AXF",
    r"C:\Workspace\DemoWorkspace\vendor\DemoMCU_SDK\Examples\HAL_Driver\Project\keil\Objects\demo.axf")
SVD = _default("UI_SVD",
    r"C:\Workspace\DemoWorkspace\vendor\DemoMCU_SDK\Libraries\CMSIS\Device\Vendor\SVD\DemoMCU.svd")
HEX = _default("UI_HEX",
    r"C:\Workspace\DemoWorkspace\workspace\DemoMCU_BOOTLOADER\Bootloader_app\dist\demo.hex")

EDGE = os.environ.get("UI_EDGE", r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")

def require(name, val):
    if not val:
        raise SystemExit(f"missing required file for {name}; set env UI_{name}")
    return val
