/**
 * Where this grid's API lives.
 *
 * A host says so with `data-api` on the element. One that says nothing gets
 * the address fugantt has always used, built from the project's id.
 */
export function apiBase(given: string | undefined, projectId: string): string {
  const base = (given ?? "").trim().replace(/\/+$/, "");

  return base !== "" ? base : `/api/projects/${encodeURIComponent(projectId)}`;
}
