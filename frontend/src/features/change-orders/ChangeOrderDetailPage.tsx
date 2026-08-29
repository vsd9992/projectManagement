import { Link, useParams } from 'react-router-dom'
import { useGetChangeOrder } from '../../api/generated/change-orders/change-orders'

export function ChangeOrderDetailPage() {
  const { id } = useParams<{ id: string }>()
  const result = useGetChangeOrder(id ?? '')

  if (result.isPending) {
    return (
      <div className="page">
        <p>Loading…</p>
      </div>
    )
  }

  if (result.data?.status !== 200) {
    return (
      <div className="page">
        <p className="form-error">{result.data?.data.error ?? 'Failed to load change order.'}</p>
      </div>
    )
  }

  const co = result.data.data

  return (
    <div className="page">
      <Link to={`/projects/${co.project_id}`}>&larr; Project</Link>
      <h1>{co.title}</h1>
      <dl className="detail-list">
        <dt>Status</dt>
        <dd>{co.status}</dd>
        <dt>Cost impact</dt>
        <dd>{co.cost_impact}</dd>
        <dt>Description</dt>
        <dd>{co.description ?? '—'}</dd>
        <dt>Created</dt>
        <dd>{new Date(co.created_at).toLocaleString()}</dd>
      </dl>

      <h2>Line items</h2>
      <table>
        <thead>
          <tr>
            <th>Description</th>
            <th>Quantity</th>
            <th>Unit</th>
            <th>Unit rate</th>
            <th>Amount</th>
            <th>Removed?</th>
          </tr>
        </thead>
        <tbody>
          {co.line_items.map((li) => (
            <tr key={li.id}>
              <td>{li.description}</td>
              <td>{li.quantity}</td>
              <td>{li.unit}</td>
              <td>{li.unit_rate}</td>
              <td>{li.amount}</td>
              <td>{li.removed ? 'Yes' : 'No'}</td>
            </tr>
          ))}
          {co.line_items.length === 0 && (
            <tr>
              <td colSpan={6}>No line item changes.</td>
            </tr>
          )}
        </tbody>
      </table>

      <h2>Workstream additions</h2>
      <ul className="workstream-list">
        {co.add_workstreams.map((w) => (
          <li key={w.id}>
            <span className="workstream-badge">{w.workstream_type}</span>
          </li>
        ))}
        {co.add_workstreams.length === 0 && <li>None.</li>}
      </ul>
    </div>
  )
}
