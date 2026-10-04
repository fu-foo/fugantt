/**
 * Where the grid's API lives. No browser needed: this is string handling, and
 * the one place a host's markup can send every request to the wrong address.
 *
 *   npm run test:unit
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { build } from "esbuild";

const { outputFiles } = await build({
  entryPoints: [new URL("../src/base.ts", import.meta.url).pathname],
  bundle: true,
  format: "esm",
  write: false,
});
const { apiBase } = await import(
  `data:text/javascript;base64,${Buffer.from(outputFiles[0].text).toString("base64")}`
);

test("with nothing said, the address is the one fugantt has always used", () => {
  assert.equal(apiBase(undefined, "abc"), "/api/projects/abc");
  assert.equal(apiBase("", "abc"), "/api/projects/abc");
  assert.equal(apiBase("   ", "abc"), "/api/projects/abc");
});

test("the project's id is encoded on the way into the default address", () => {
  assert.equal(apiBase(undefined, "計画 1"), "/api/projects/%E8%A8%88%E7%94%BB%201");
  assert.equal(apiBase(undefined, "a/b"), "/api/projects/a%2Fb");
});

test("a host's own address is used as it stands", () => {
  assert.equal(apiBase("/g/api/plans/demo", "abc"), "/g/api/plans/demo");
  assert.equal(apiBase("https://example.test/x/abc", "abc"), "https://example.test/x/abc");
});

test("a trailing slash does not become a doubled one", () => {
  assert.equal(apiBase("/g/api/plans/demo/", "abc"), "/g/api/plans/demo");
  assert.equal(apiBase("/g/api/plans/demo///", "abc"), "/g/api/plans/demo");
  assert.equal(apiBase("  /g/api/plans/demo/  ", "abc"), "/g/api/plans/demo");
});
