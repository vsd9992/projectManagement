import { Link, useParams } from 'react-router-dom'
import { useGetQuotation } from '../../api/generated/quotations/quotations'

export function QuotationDetailPage() {
  const { id } = useParams<{ id: string }>()
  const quotation = useGetQuotation(id ?? '')

  if (quotation.isPending) {
    return (
      <div className="page">
        <p>Loading…</p>
      </div>
    )
  }

  if (quotation.data?.status !== 200) {
    return (
      <div className="page">
        <p className="form-error">{quotation.data?.data.error ?? 'Failed to load quotation.'}</p>
      </div>
    )
  }

  const q = quotation.data.data
  const total = q.line_items.reduce((sum, li) => sum + Number(li.amount), 0)

  return (
    <div className="page">
      <Link to={`/projects/${q.project_id}`}>&larr; Project</Link>
      <h1>
        Quotation v{q.version} <span className="workstream-badge">{q.status}</span>
      </h1>
      <p>Created {new Date(q.created_at).toLocaleString()}</p>

      <table>
        <thead>
          <tr>
            <th>Description</th>
            <th>Quantity</th>
            <th>Unit</th>
            <th>Unit rate</th>
            <th>Amount</th>
          </tr>
        </thead>
        <tbody>
          {q.line_items.map((li) => (
            <tr key={li.id}>
              <td>{li.description}</td>
              <td>{li.quantity}</td>
              <td>{li.unit}</td>
              <td>{li.unit_rate}</td>
              <td>{li.amount}</td>
            </tr>
          ))}
          {q.line_items.length === 0 && (
            <tr>
              <td colSpan={5}>No line items.</td>
            </tr>
          )}
        </tbody>
        <tfoot>
          <tr>
            <td colSpan={4}>Total</td>
            <td>{total.toFixed(2)}</td>
          </tr>
        </tfoot>
      </table>
    </div>
  )
}
