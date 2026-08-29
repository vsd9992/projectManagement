import { Link } from 'react-router-dom'
import { useListMyProjects } from '../../api/generated/client-portal/client-portal'

export function ClientProjectsPage() {
  const list = useListMyProjects()

  return (
    <div className="page">
      <h1>My Projects</h1>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Billing method</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((p) => (
              <tr key={p.id}>
                <td>
                  <Link to={`/client/projects/${p.id}`}>{p.name}</Link>
                </td>
                <td>{p.billing_method}</td>
                <td>{new Date(p.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={3}>No projects yet.</td>
              </tr>
            )}
          </tbody>
        </table>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </div>
  )
}
