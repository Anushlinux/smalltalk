import process from "node:process";
import { fileURLToPath } from "node:url";
import { loadEnv } from "vite";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const fileEnvironment = loadEnv("production", repoRoot, "");
const environment = {
  ...fileEnvironment,
  ...process.env,
};

const requiredValues = [
  "VITE_SUPABASE_URL",
  "VITE_SUPABASE_PUBLISHABLE_KEY",
];

const missingValues = requiredValues.filter(
  (name) => !environment[name]?.trim(),
);

if (missingValues.length > 0) {
  console.error(
    `Authentication build configuration is missing: ${missingValues.join(", ")}.`,
  );
  console.error(
    "Local builds may define these public client values in .env. GitHub release builds must define matching repository Actions variables.",
  );
  process.exit(1);
}

let supabaseUrl;
try {
  supabaseUrl = new URL(environment.VITE_SUPABASE_URL);
} catch {
  console.error("VITE_SUPABASE_URL must be a valid URL.");
  process.exit(1);
}

if (
  supabaseUrl.protocol !== "https:" ||
  !supabaseUrl.hostname.endsWith(".supabase.co")
) {
  console.error(
    "VITE_SUPABASE_URL must use HTTPS and point to a supabase.co project.",
  );
  process.exit(1);
}

console.log("Supabase public client configuration is available for this build.");
