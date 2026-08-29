import { Link, useParams } from 'react-router-dom'
import { useGetProject } from '../../api/generated/projects/projects'

export function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>()
  const project = useGetProject(id ?? '')

  if (project.isPending) {
    return (
      <div className="page">
        <p>Loading…</p>
      </div>
    )
  }

  if (project.data?.status !== 200) {
    return (
      <div className="page">
        <p className="form-error">{project.data?.data.error ?? 'Failed to load project.'}</p>
        <Link to="/">Back to projects</Link>
      </div>
    )
  }

  const p = project.data.data

  return (
    <div className="page">
      <Link to="/">&larr; Projects</Link>
      <h1>{p.name}</h1>
      <dl className="detail-list">
        <dt>Billing method</dt>
        <dd>{p.billing_method}</dd>
        <dt>Created</dt>
        <dd>{new Date(p.created_at).toLocaleString()}</dd>
      </dl>

      <h2>Workstreams</h2>
      <ul className="workstream-list">
        {p.workstreams.map((w) => (
          <li key={w.id}>
            <span className="workstream-badge">{w.workstream_type}</span>
            <span>{w.status}</span>
          </li>
        ))}
        {p.workstreams.length === 0 && <li>No workstreams enabled.</li>}
      </ul>
    </div>
  )
}
