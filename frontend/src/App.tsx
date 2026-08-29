import { Navigate, Route, Routes } from 'react-router-dom'
import { AppShell } from './app/AppShell'
import { RequireAuth } from './app/RequireAuth'
import { LoginPage } from './features/auth/LoginPage'
import { SignupPage } from './features/auth/SignupPage'
import { BusinessUnitsPage } from './features/business-units/BusinessUnitsPage'
import { ClientsPage } from './features/clients/ClientsPage'
import { ProjectDetailPage } from './features/projects/ProjectDetailPage'
import { ProjectsListPage } from './features/projects/ProjectsListPage'

function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/signup" element={<SignupPage />} />

      <Route element={<RequireAuth />}>
        <Route element={<AppShell />}>
          <Route path="/" element={<ProjectsListPage />} />
          <Route path="/projects/:id" element={<ProjectDetailPage />} />
          <Route path="/business-units" element={<BusinessUnitsPage />} />
          <Route path="/clients" element={<ClientsPage />} />
        </Route>
      </Route>

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}

export default App
