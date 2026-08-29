import { Link, NavLink, Outlet, useNavigate } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import { useLogout } from '../../api/generated/auth/auth'
import { getListMyProjectsQueryKey } from '../../api/generated/client-portal/client-portal'

export function ClientAppShell() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const logout = useLogout()

  async function handleLogout() {
    await logout.mutateAsync()
    queryClient.removeQueries({ queryKey: getListMyProjectsQueryKey() })
    navigate('/client/login', { replace: true })
  }

  return (
    <div className="app-shell">
      <header className="app-nav">
        <Link to="/client" className="app-nav-brand">
          Client Portal
        </Link>
        <nav>
          <NavLink to="/client" end>
            Projects
          </NavLink>
        </nav>
        <div className="app-nav-user">
          <button type="button" onClick={handleLogout} disabled={logout.isPending}>
            Log out
          </button>
        </div>
      </header>
      <main className="app-content">
        <Outlet />
      </main>
    </div>
  )
}
