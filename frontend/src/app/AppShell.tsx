import { Link, NavLink, Outlet, useNavigate } from 'react-router-dom'
import { useLogout } from '../api/generated/auth/auth'
import { useClearCurrentUser, useCurrentUser } from '../features/auth/useCurrentUser'

export function AppShell() {
  const current = useCurrentUser()
  const navigate = useNavigate()
  const clearCurrentUser = useClearCurrentUser()
  const logout = useLogout()

  async function handleLogout() {
    await logout.mutateAsync()
    clearCurrentUser()
    navigate('/login', { replace: true })
  }

  return (
    <div className="app-shell">
      <header className="app-nav">
        <Link to="/" className="app-nav-brand">
          Project Management
        </Link>
        <nav>
          <NavLink to="/" end>
            Projects
          </NavLink>
          <NavLink to="/leads">Leads</NavLink>
          <NavLink to="/business-units">Business Units</NavLink>
          <NavLink to="/clients">Clients</NavLink>
          <NavLink to="/vendors">Vendors</NavLink>
          <NavLink to="/notifications">Notifications</NavLink>
          {current.status === 'authenticated' && current.user.is_tenant_admin && (
            <NavLink to="/tenant-settings">Tenant Settings</NavLink>
          )}
        </nav>
        <div className="app-nav-user">
          {current.status === 'authenticated' && (
            <>
              <span>
                {current.user.email}
                {current.user.is_tenant_admin ? ' · admin' : ''}
              </span>
              <button type="button" onClick={handleLogout} disabled={logout.isPending}>
                Log out
              </button>
            </>
          )}
        </div>
      </header>
      <main className="app-content">
        <Outlet />
      </main>
    </div>
  )
}
