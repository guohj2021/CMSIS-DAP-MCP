#!/usr/bin/env node
const { spawn } = require("child_process");
const path = require("path");

const SCOPE = "@guohj2021";
const platformMap = { win32: "win32", linux: "linux", darwin: "darwin" };
const archMap = { x64: "x64", arm64: "arm64", ia32: "ia32" };

const platform = platformMap[process.platform];
const arch = archMap[process.arch];
if (!platform || !arch) {
  console.error(`cmsis-dap-mcp: unsupported platform ${process.platform}/${process.arch}`);
  process.exit(1);
}

const pkgName = `${SCOPE}/cmsis-dap-mcp-${platform}-${arch}`;
let pkgJsonPath;
try {
  pkgJsonPath = require.resolve(`${pkgName}/package.json`);
} catch {
  console.error(`cmsis-dap-mcp: platform package ${pkgName} not installed`);
  process.exit(1);
}

const bin = path.join(path.dirname(pkgJsonPath), "bin", platform === "win32" ? "cmsis-dap-mcp.exe" : "cmsis-dap-mcp");
const child = spawn(bin, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 0);
  }
});
