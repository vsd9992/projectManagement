import { Navigate, Outlet } from 'react-router-dom'
import { useListMyProjects } from '../../api/generated/client-portal/client-portal'

/**
 * There is no client-facing "who am I" endpoint (unlike internal
 * GET /api/auth/me), so session state is inferred from whether the projects
 * list resolves 200 or 401 — the same 401-resolves-not-throws contract every
 * other endpoint uses (see customFetch.ts).
 */
export function RequireClientAuth() {
  const projects = useListMyProjects()

  if (projects.isPending) {
    return (
      <div className="page-center">
        <p>Loading…</p>
      </div>
    )
  }

  if (projects.data?.status !== 200) {
    return <Navigate to="/client/login" replace />
  }

  return <Outlet />
}
