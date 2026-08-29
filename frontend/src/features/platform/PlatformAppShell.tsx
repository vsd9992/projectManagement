import { Link, NavLink, Outlet, useNavigate } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import { getListTenantsQueryKey, usePlatformLogout } from '../../api/generated/platform/platform'

export function PlatformAppShell() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const logout = usePlatformLogout()

  async function handleLogout() {
    await logout.mutateAsync()
    queryClient.removeQueries({ queryKey: getListTenantsQueryKey() })
    navigate('/platform/login', { replace: true })
  }

  return (
    <div className="app-shell">
      <header className="app-nav">
        <Link to="/platform" className="app-nav-brand">
          Platform Admin
        </Link>
        <nav>
          <NavLink to="/platform" end>
            Tenants
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
