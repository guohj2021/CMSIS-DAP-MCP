# -*- coding: utf-8 -*-
"""Shared config for UI functional tests (Playwright/Edge).

Environment variables:
  UI_URL    - web server base URL (default http://127.0.0.1:18080/)
  UI_AXF    - ELF/AXF file for symbol tests (no default; required by those tests)
  UI_SVD    - SVD file for peripheral tests (no default; required by those tests)
  UI_HEX    - HEX/BIN file for flash tests (no default; required by those tests)
  UI_EDGE   - path to msedge.exe (auto-detected if unset)
"""
import os

URL = os.environ.get("UI_URL", "http://127.0.0.1:18080/")

AXF = os.environ.get("UI_AXF")
SVD = os.environ.get("UI_SVD")
HEX = os.environ.get("UI_HEX")

EDGE = os.environ.get("UI_EDGE", r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")

def require(name, val):
    if not val:
        raise SystemExit(f"missing required file for {name}; set env UI_{name}")
    return val
