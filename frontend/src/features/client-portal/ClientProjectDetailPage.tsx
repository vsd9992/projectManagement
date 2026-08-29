import { useState } from 'react'
import { useParams } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListMyChangeOrdersQueryKey,
  getListProjectDesignAssetsQueryKey,
  useApproveChangeOrder,
  useApproveDesignRevision,
  useApproveQuotation,
  useListMyChangeOrders,
  useListMyProjects,
  useListProjectDesignAssets,
  useListProjectInvoices,
  useListProjectQuotations,
  useRejectChangeOrder,
  useRejectDesignRevision,
  useRejectQuotation,
  getListProjectQuotationsQueryKey,
} from '../../api/generated/client-portal/client-portal'

function QuotationsSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListProjectQuotations(projectId)
  const approve = useApproveQuotation()
  const reject = useRejectQuotation()
  const [error, setError] = useState<string | null>(null)

  async function handleDecision(id: string, approved: boolean) {
    setError(null)
    const result = approved
      ? await approve.mutateAsync({ id, data: {} })
      : await reject.mutateAsync({ id, data: {} })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListProjectQuotationsQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Quotations</h3>
      {error && <p className="form-error">{error}</p>}
      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="workstream-list">
          {list.data.data.map((q) => (
            <li key={q.id}>
              <span className="workstream-badge">{q.status}</span>
              <span> v{q.version}</span>
              {(q.status === 'draft' || q.status === 'sent') && (
                <>
                  <button type="button" onClick={() => handleDecision(q.id, true)}>
                    Approve
                  </button>{' '}
                  <button type="button" onClick={() => handleDecision(q.id, false)}>
                    Reject
                  </button>
                </>
              )}
            </li>
          ))}
          {list.data.data.length === 0 && <li>No quotations yet.</li>}
        </ul>
      )}
    </div>
  )
}

function DesignAssetsSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListProjectDesignAssets(projectId)
  const approve = useApproveDesignRevision()
  const reject = useRejectDesignRevision()
  const [error, setError] = useState<string | null>(null)

  async function handleDecision(id: string, approved: boolean) {
    setError(null)
    const result = approved
      ? await approve.mutateAsync({ id, data: {} })
      : await reject.mutateAsync({ id, data: {} })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListProjectDesignAssetsQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Design</h3>
      {error && <p className="form-error">{error}</p>}
      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="design-asset-list">
          {list.data.data.map((asset) => (
            <li key={asset.id}>
              <h4>{asset.title}</h4>
              <ul className="workstream-list">
                {asset.revisions.map((rev) => (
                  <li key={rev.id}>
                    <span className="workstream-badge">{rev.status}</span>
                    {rev.notes && <span> {rev.notes}</span>}
                    {rev.status === 'submitted' && (
                      <>
                        {' '}
                        <button type="button" onClick={() => handleDecision(rev.id, true)}>
                          Approve
                        </button>{' '}
                        <button type="button" onClick={() => handleDecision(rev.id, false)}>
                          Reject
                        </button>
                      </>
                    )}
                  </li>
                ))}
                {asset.revisions.length === 0 && <li>No revisions yet.</li>}
              </ul>
            </li>
          ))}
          {list.data.data.length === 0 && <li>No design assets yet.</li>}
        </ul>
      )}
    </div>
  )
}

function ChangeOrdersSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListMyChangeOrders(projectId)
  const approve = useApproveChangeOrder()
  const reject = useRejectChangeOrder()
  const [error, setError] = useState<string | null>(null)

  async function handleDecision(id: string, approved: boolean) {
    setError(null)
    const result = approved
      ? await approve.mutateAsync({ id, data: {} })
      : await reject.mutateAsync({ id, data: {} })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListMyChangeOrdersQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Change Orders</h3>
      {error && <p className="form-error">{error}</p>}
      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="workstream-list">
          {list.data.data.map((co) => (
            <li key={co.id}>
              <span className="workstream-badge">{co.status}</span>
              <span>
                {' '}
                {co.title} (cost impact: {co.cost_impact})
              </span>
              {co.status === 'pending_client_approval' && (
                <>
                  {' '}
                  <button type="button" onClick={() => handleDecision(co.id, true)}>
                    Approve
                  </button>{' '}
                  <button type="button" onClick={() => handleDecision(co.id, false)}>
                    Reject
                  </button>
                </>
              )}
            </li>
          ))}
          {list.data.data.length === 0 && <li>No change orders yet.</li>}
        </ul>
      )}
    </div>
  )
}

function InvoicesSubsection({ projectId }: { projectId: string }) {
  const list = useListProjectInvoices(projectId)

  return (
    <div>
      <h3>Invoices</h3>
      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Net payable</th>
              <th>Status</th>
              <th>Raised</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((inv) => (
              <tr key={inv.id}>
                <td>{inv.net_payable}</td>
                <td>{inv.status}</td>
                <td>{new Date(inv.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={3}>No invoices yet.</td>
              </tr>
            )}
          </tbody>
        </table>
      )}
    </div>
  )
}

export function ClientProjectDetailPage() {
  const { id } = useParams<{ id: string }>()
  const projects = useListMyProjects()

  if (projects.isPending) {
    return (
      <div className="page">
        <p>Loading…</p>
      </div>
    )
  }

  const project =
    projects.data?.status === 200 ? projects.data.data.find((p) => p.id === id) : undefined

  if (!project) {
    return (
      <div className="page">
        <p className="form-error">Project not found.</p>
      </div>
    )
  }

  return (
    <div className="page">
      <h1>{project.name}</h1>
      <dl className="detail-list">
        <dt>Billing method</dt>
        <dd>{project.billing_method}</dd>
      </dl>

      <QuotationsSubsection projectId={project.id} />
      <DesignAssetsSubsection projectId={project.id} />
      <ChangeOrdersSubsection projectId={project.id} />
      <InvoicesSubsection projectId={project.id} />
    </div>
  )
}
