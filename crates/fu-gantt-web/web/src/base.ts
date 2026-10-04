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

/**
 * The query a host wants on every request, without the punctuation in front.
 *
 * A host that shows the chart over a filtered set of rows says which set with
 * `data-query`. It is the host's own string and is not read here — only tidied,
 * so that `?status=open` and `status=open` mean the same thing.
 */
export function cleanQuery(given: string | undefined): string {
  return (given ?? "").trim().replace(/^[?&]+/, "").trim();
}

/** `url` with the host's query on it, or `url` as it stands when there is none. */
export function withQuery(url: string, query: string): string {
  return query === "" ? url : `${url}?${query}`;
}

/**
 * Where a row leads, when the host has said rows lead somewhere.
 *
 * `{id}` in the template is the row's id. Only an address on this site (`/…`)
 * or a full `http`/`https` one is taken: the template goes into an `href`, and
 * anything else there is either a mistake or an attack.
 */
export function rowLink(template: string | undefined, id: string): string | null {
  const given = (template ?? "").trim();
  const local = given.startsWith("/") && !given.startsWith("//");

  if (!local && !/^https?:\/\//i.test(given)) return null;

  return given.split("{id}").join(encodeURIComponent(id));
}
