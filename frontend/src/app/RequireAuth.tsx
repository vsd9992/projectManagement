import { Navigate, Outlet } from 'react-router-dom'
import { useCurrentUser } from '../features/auth/useCurrentUser'

export function RequireAuth() {
  const current = useCurrentUser()

  if (current.status === 'loading') {
    return (
      <div className="page-center">
        <p>Loading…</p>
      </div>
    )
  }

  if (current.status === 'unauthenticated') {
    return <Navigate to="/login" replace />
  }

  return <Outlet />
}
