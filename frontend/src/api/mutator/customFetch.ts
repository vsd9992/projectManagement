/**
 * orval's fetch-client mutator. `credentials: 'include'` is what actually
 * carries the httpOnly session cookie across the Vite dev-server proxy
 * (:5173 -> backend :8081) — without it, login would silently never
 * persist.
 *
 * orval generates a status-discriminated response type per operation (e.g.
 * `{ data: MeResponse; status: 200 } | { data: ErrorResponse; status: 401 }`)
 * whenever a path documents more than one response status — which every
 * endpoint here does (200 + at least 401/403/404). That means this mutator
 * must resolve (not throw) for expected error statuses like 401, returning
 * `{ data, status, headers }` uniformly so callers narrow on `.status`
 * themselves — see `features/auth/useCurrentUser.ts` for the pattern. A
 * thrown error here is reserved for a genuine network failure, which
 * `fetch()` already throws for on its own.
 */
export const customFetch = async <T>(url: string, options: RequestInit): Promise<T> => {
  const response = await fetch(url, {
    ...options,
    credentials: 'include',
  })

  const text = await response.text()
  const data = text ? JSON.parse(text) : undefined

  return {
    data,
    status: response.status,
    headers: response.headers,
  } as T
}

export default customFetch
