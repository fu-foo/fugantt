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
const { apiBase, cleanQuery, withQuery, rowLink } = await import(
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

test("with no query, an address is left exactly as it was", () => {
  assert.equal(cleanQuery(undefined), "");
  assert.equal(cleanQuery(""), "");
  assert.equal(cleanQuery("   "), "");
  assert.equal(withQuery("/api/projects/abc/grid", cleanQuery(undefined)), "/api/projects/abc/grid");
});

test("a host's query rides on every address", () => {
  const query = cleanQuery("status=open&assignee=%E4%BD%90%E8%97%A4");

  assert.equal(
    withQuery("/g/api/grid", query),
    "/g/api/grid?status=open&assignee=%E4%BD%90%E8%97%A4",
  );
  assert.equal(withQuery("/g/api/tasks/t-1/patch", query).split("?").length, 2);
});

test("a query written with its own ? or & does not double them", () => {
  assert.equal(cleanQuery("?status=open"), "status=open");
  assert.equal(cleanQuery("&status=open"), "status=open");
  assert.equal(cleanQuery("  ??&status=open  "), "status=open");
  assert.equal(cleanQuery("?"), "");
  assert.equal(withQuery("/x", cleanQuery("?")), "/x");
});

test("a row's link is the template with the row's id in it", () => {
  assert.equal(rowLink("/issues/by-id/{id}", "t-1"), "/issues/by-id/t-1");
  assert.equal(rowLink("https://example.test/i/{id}/view", "t-1"), "https://example.test/i/t-1/view");
  assert.equal(rowLink("/a/{id}/b/{id}", "t-1"), "/a/t-1/b/t-1");
  // No place for the id: every row goes to the same page, as the host said.
  assert.equal(rowLink("/issues", "t-1"), "/issues");
});

test("an id is encoded on the way into the link", () => {
  assert.equal(rowLink("/i/{id}", "a/b"), "/i/a%2Fb");
  assert.equal(rowLink("/i/{id}", "a b"), "/i/a%20b");
  assert.equal(rowLink("/i/{id}", "#1"), "/i/%231");
});

test("no template, or one that is not an address, gives no link", () => {
  assert.equal(rowLink(undefined, "t-1"), null);
  assert.equal(rowLink("", "t-1"), null);
  assert.equal(rowLink("   ", "t-1"), null);
  assert.equal(rowLink("javascript:alert({id})", "t-1"), null);
  assert.equal(rowLink("JavaScript:alert(1)", "t-1"), null);
  assert.equal(rowLink("data:text/html,{id}", "t-1"), null);
  assert.equal(rowLink("issues/{id}", "t-1"), null);
  assert.equal(rowLink("//evil.test/{id}", "t-1"), null);
});
