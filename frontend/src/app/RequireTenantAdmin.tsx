import { Navigate, Outlet } from 'react-router-dom'
import { useCurrentUser } from '../features/auth/useCurrentUser'

export function RequireTenantAdmin() {
  const current = useCurrentUser()

  if (current.status !== 'authenticated') {
    return null
  }

  if (!current.user.is_tenant_admin) {
    return <Navigate to="/" replace />
  }

  return <Outlet />
}
