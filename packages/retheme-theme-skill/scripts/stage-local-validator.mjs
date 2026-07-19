import { chmodSync, cpSync, existsSync, mkdirSync } from "node:fs";
import { arch, platform } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repository = resolve(packageDirectory, "../..");
const keys = {
  "darwin:arm64": "darwin-arm64",
  "darwin:x64": "darwin-x64",
  "win32:x64": "win32-x64",
  "linux:x64": "linux-x64"
};
const key = keys[`${platform()}:${arch()}`];
if (!key) throw new Error(`Unsupported local platform: ${platform()}-${arch()}`);

const executable = platform() === "win32" ? "retheme-theme-validator.exe" : "retheme-theme-validator";
const source = resolve(repository, "crates/theme-validator/target/release", executable);
if (!existsSync(source)) {
  throw new Error(`Build the release validator first: ${source}`);
}
const destination = resolve(packageDirectory, "native", key, executable);
mkdirSync(dirname(destination), { recursive: true });
cpSync(source, destination);
if (platform() !== "win32") chmodSync(destination, 0o755);
console.log(destination);
