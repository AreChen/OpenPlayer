import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const binary = path.join(root, "target", "debug", process.platform === "win32" ? "window-smoke.exe" : "window-smoke");
const environment = { ...process.env };
if (process.platform === "win32") {
  const key = Object.keys(environment).find((key) => key.toLowerCase() === "path") ?? "PATH";
  environment[key] = `${path.join(root, "vendor/native/mpv/windows-x64")};${environment[key] ?? ""}`;
}
for (const target of ["overlay", "main"]) {
  console.log(`Native window smoke: close ${target}`);
  await new Promise((resolve, reject) => {
    const child = spawn(binary, [target], { cwd: root, env: environment, stdio: "inherit", windowsHide: true });
    const timeout = setTimeout(() => { child.kill(); reject(new Error(`window smoke timed out: ${target}`)); }, 55000);
    child.once("error", (error) => { clearTimeout(timeout); reject(error); });
    child.once("exit", (code, signal) => {
      clearTimeout(timeout);
      if (code === 0) resolve();
      else reject(new Error(`window smoke failed: ${target} (exit ${code}, signal ${signal})`));
    });
  });
}
