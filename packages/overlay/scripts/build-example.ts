import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const GENERATED_START = "<!-- LICHORA_OVERLAY_BUNDLE_START -->";
const GENERATED_END = "<!-- LICHORA_OVERLAY_BUNDLE_END -->";
const packageDirectory = resolve(import.meta.dir, "..");
const defaultExamplePath = resolve(packageDirectory, "examples/pass-map.html");
const entrypoint = resolve(packageDirectory, "examples/pass-map-entry.ts");

export interface BuildExampleOptions {
    templatePath?: string;
    outputPath?: string;
}

export async function buildExample(options: BuildExampleOptions = {}): Promise<string> {
    const templatePath = resolve(options.templatePath ?? defaultExamplePath);
    const outputPath = resolve(options.outputPath ?? templatePath);
    const template = await readFile(templatePath, "utf8");
    const build = await Bun.build({
        entrypoints: [entrypoint],
        target: "browser",
        format: "iife",
        minify: false,
        sourcemap: "none",
        write: false,
    });

    if (!build.success || build.outputs.length !== 1) {
        throw new Error(`overlay example bundle failed: ${build.logs.map(String).join("\n")}`);
    }

    const bundle = await build.outputs[0].text();
    const generatedBlock = createGeneratedBlock(bundle);
    const html = replaceScriptBlock(template, generatedBlock);
    await Bun.write(outputPath, html);
    return outputPath;
}

function createGeneratedBlock(bundle: string): string {
    const indentedBundle = bundle
        .trimEnd()
        .split(/\r?\n/)
        .map((line) => (line ? `    ${line}` : ""))
        .join("\n");
    return `  ${GENERATED_START}\n  <script>\n${indentedBundle}\n  </script>\n  ${GENERATED_END}`;
}

function replaceScriptBlock(template: string, generatedBlock: string): string {
    const generatedPattern = new RegExp(
        `  ${GENERATED_START}[\\s\\S]*?  ${GENERATED_END}`,
        "m",
    );
    if (generatedPattern.test(template)) {
        return template.replace(generatedPattern, generatedBlock);
    }

    const scriptPattern = /  <script(?:\s+type="module")?>[\s\S]*?<\/script>(?=\s*<\/body>)/g;
    const scripts = [...template.matchAll(scriptPattern)];
    if (scripts.length !== 1) {
        throw new Error(`expected one trailing script block in ${scripts.length === 0 ? "template" : "template, found multiple"}`);
    }
    return template.replace(scriptPattern, generatedBlock);
}

if (import.meta.main) {
    const outputPath = process.argv[2];
    const result = await buildExample(outputPath ? { templatePath: outputPath, outputPath } : undefined);
    console.log(`Built self-contained overlay example: ${result}`);
}
