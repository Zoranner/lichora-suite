import { access, readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packagePath = resolve(packageDirectory, "package.json");
const packageJson = JSON.parse(await readFile(packagePath, "utf8"));

if (packageJson.exports?.["."] !== "./src/index.ts") {
    throw new Error("package.json exports['.'] must point to ./src/index.ts");
}

for (const entry of ["src", "fixtures", "examples", "scripts", "tsconfig.json", "README.md"]) {
    if (!packageJson.files?.includes(entry)) {
        throw new Error(`package.json files is missing '${entry}'`);
    }
    await access(resolve(packageDirectory, entry));
}

await access(resolve(packageDirectory, "src/index.ts"));
console.log(`Package contract passed: ${packageJson.name}@${packageJson.version}`);
