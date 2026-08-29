import { useQueryClient } from '@tanstack/react-query'
import { getMeQueryKey, me, useMe } from '../../api/generated/auth/auth'
import type { MeResponse } from '../../api/generated/api.schemas'

export type CurrentUserState =
  | { status: 'loading' }
  | { status: 'authenticated'; user: MeResponse }
  | { status: 'unauthenticated' }

/**
 * The single source of truth for "am I logged in" — there's no client-stored
 * token to check (the session cookie is httpOnly), so this always reflects
 * a live `GET /api/auth/me` call. The endpoint resolves (not throws) on a
 * 401 — see customFetch.ts — so authentication state is read from
 * `.data.status`, not React Query's error state.
 */
export function useCurrentUser(): CurrentUserState {
  const query = useMe()

  if (query.isPending) {
    return { status: 'loading' }
  }
  if (query.data?.status === 200) {
    return { status: 'authenticated', user: query.data.data }
  }
  return { status: 'unauthenticated' }
}

/**
 * Call right after login/signup succeeds, before navigating into a
 * protected route. Deliberately does NOT use `queryClient.invalidateQueries`
 * — invalidating a query nobody is subscribed to yet (LoginPage/SignupPage
 * don't render RequireAuth) only marks it stale, so the *next* mount's own
 * fetch races the `navigate()` call, and under dev StrictMode's
 * mount→unmount→remount cycle that race can abort every attempt before any
 * of them resolve, leaving RequireAuth stuck seeing no data and bouncing
 * back to /login even though the login itself succeeded. Awaiting the real
 * `me()` call and seeding the cache directly means RequireAuth's first
 * render after navigation already has correct, settled data.
 */
export function useRefreshCurrentUser() {
  const queryClient = useQueryClient()
  return async () => {
    const result = await me()
    queryClient.setQueryData(getMeQueryKey(), result)
  }
}

/** Call after logout — clears the cached identity so the next protected-route mount refetches fresh (and correctly gets 401, since the cookie is now cleared server-side). */
export function useClearCurrentUser() {
  const queryClient = useQueryClient()
  return () => queryClient.removeQueries({ queryKey: getMeQueryKey() })
}
