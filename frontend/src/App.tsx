import { Navigate, Route, Routes } from 'react-router-dom'
import { AppShell } from './app/AppShell'
import { RequireAuth } from './app/RequireAuth'
import { RequireTenantAdmin } from './app/RequireTenantAdmin'
import { LoginPage } from './features/auth/LoginPage'
import { SignupPage } from './features/auth/SignupPage'
import { BusinessUnitsPage } from './features/business-units/BusinessUnitsPage'
import { ClientsPage } from './features/clients/ClientsPage'
import { LeadsPage } from './features/leads/LeadsPage'
import { ProjectDetailPage } from './features/projects/ProjectDetailPage'
import { ProjectsListPage } from './features/projects/ProjectsListPage'
import { QuotationDetailPage } from './features/quotations/QuotationDetailPage'
import { VendorsPage } from './features/procurement/VendorsPage'
import { ChangeOrderDetailPage } from './features/change-orders/ChangeOrderDetailPage'
import { NotificationsPage } from './features/notifications/NotificationsPage'
import { TenantSettingsPage } from './features/tenant-settings/TenantSettingsPage'
import { ClientLoginPage } from './features/client-portal/ClientLoginPage'
import { ClientAppShell } from './features/client-portal/ClientAppShell'
import { RequireClientAuth } from './features/client-portal/RequireClientAuth'
import { ClientProjectsPage } from './features/client-portal/ClientProjectsPage'
import { ClientProjectDetailPage } from './features/client-portal/ClientProjectDetailPage'
import { PlatformLoginPage } from './features/platform/PlatformLoginPage'
import { PlatformAppShell } from './features/platform/PlatformAppShell'
import { RequirePlatformAuth } from './features/platform/RequirePlatformAuth'
import { PlatformTenantsPage } from './features/platform/PlatformTenantsPage'

function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/signup" element={<SignupPage />} />

      <Route element={<RequireAuth />}>
        <Route element={<AppShell />}>
          <Route path="/" element={<ProjectsListPage />} />
          <Route path="/leads" element={<LeadsPage />} />
          <Route path="/projects/:id" element={<ProjectDetailPage />} />
          <Route path="/quotations/:id" element={<QuotationDetailPage />} />
          <Route path="/change-orders/:id" element={<ChangeOrderDetailPage />} />
          <Route path="/business-units" element={<BusinessUnitsPage />} />
          <Route path="/clients" element={<ClientsPage />} />
          <Route path="/vendors" element={<VendorsPage />} />
          <Route path="/notifications" element={<NotificationsPage />} />
          <Route element={<RequireTenantAdmin />}>
            <Route path="/tenant-settings" element={<TenantSettingsPage />} />
          </Route>
        </Route>
      </Route>

      <Route path="/client/login" element={<ClientLoginPage />} />
      <Route element={<RequireClientAuth />}>
        <Route element={<ClientAppShell />}>
          <Route path="/client" element={<ClientProjectsPage />} />
          <Route path="/client/projects/:id" element={<ClientProjectDetailPage />} />
        </Route>
      </Route>

      <Route path="/platform/login" element={<PlatformLoginPage />} />
      <Route element={<RequirePlatformAuth />}>
        <Route element={<PlatformAppShell />}>
          <Route path="/platform" element={<PlatformTenantsPage />} />
        </Route>
      </Route>

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}

export default App
