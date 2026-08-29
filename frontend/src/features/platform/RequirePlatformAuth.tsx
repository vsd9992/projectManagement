import { Navigate, Outlet } from 'react-router-dom'
import { useListTenants } from '../../api/generated/platform/platform'

/**
 * No platform-admin "who am I" endpoint exists either, so session state is
 * inferred from list_tenants resolving 200 vs 401 — same pattern as
 * client-portal/RequireClientAuth.
 */
export function RequirePlatformAuth() {
  const tenants = useListTenants()

  if (tenants.isPending) {
    return (
      <div className="page-center">
        <p>Loading…</p>
      </div>
    )
  }

  if (tenants.data?.status !== 200) {
    return <Navigate to="/platform/login" replace />
  }

  return <Outlet />
}
