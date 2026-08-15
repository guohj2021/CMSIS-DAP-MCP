#!/usr/bin/env node
const { spawn } = require("child_process");
const path = require("path");

const platformMap = {
  win32: "win32",
  linux: "linux",
  darwin: "darwin",
};
const archMap = {
  x64: "x64",
  arm64: "arm64",
};

const platform = platformMap[process.platform];
const arch = archMap[process.arch];
if (!platform || !arch) {
  console.error(`cmsis-dap-mcp: unsupported platform ${process.platform}/${process.arch}`);
  process.exit(1);
}

const pkgName = `cmsis-dap-mcp-${platform}-${arch}`;
let binaryPath;
try {
  binaryPath = require.resolve(pkgName);
} catch {
  console.error(`cmsis-dap-mcp: platform package ${pkgName} not installed`);
  process.exit(1);
}

const bin = path.join(path.dirname(binaryPath), "bin", platform === "win32" ? "cmsis-dap-mcp.exe" : "cmsis-dap-mcp");
const child = spawn(bin, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 0);
  }
});