import { mkdirSync, writeFileSync } from "node:fs";

mkdirSync("src-tauri/config", { recursive: true });
writeFileSync("src-tauri/config/api.toml", [
  'base_url = "https://127.0.0.1"',
  'secret_id = "retheme-we-local-features"',
  'secret_key = "retheme-we-local-features"',
  "",
].join("\n"));
writeFileSync("src-tauri/config/security.toml", [
  'package_key = "retheme-we-local-features-00000000000000000000000000000000"',
  'license_public_key = "0000000000000000000000000000000000000000000000000000000000000000"',
  'compatibility_public_key = "0000000000000000000000000000000000000000000000000000000000000000"',
  'theme_public_key = "0000000000000000000000000000000000000000000000000000000000000000"',
  "",
].join("\n"));
