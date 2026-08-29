import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { useGetProject } from '../../api/generated/projects/projects'
import {
  getListQuotationsQueryKey,
  useCreateQuotation,
  useListQuotations,
} from '../../api/generated/quotations/quotations'
import {
  getListDesignAssetsQueryKey,
  getListDesignRevisionsQueryKey,
  useCreateDesignAsset,
  useListDesignAssets,
  useListDesignRevisions,
  useSubmitDesignRevision,
} from '../../api/generated/design/design'
import type { DesignAssetModel, LineItemInput } from '../../api/generated/api.schemas'
import { PurchaseOrdersSection } from '../procurement/PurchaseOrdersSection'
import { ProductionTasksSection } from '../manufacturing/ProductionTasksSection'
import { SiteExecutionSection } from '../site-execution/SiteExecutionSection'
import { BillingSection } from '../billing/BillingSection'
import { ChangeOrdersSection } from '../change-orders/ChangeOrdersSection'
import { ScheduleSection } from '../schedule/ScheduleSection'

const EMPTY_LINE_ITEM: LineItemInput = { description: '', quantity: '', unit: '', unit_rate: '' }

function QuotationsSection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListQuotations(projectId)
  const create = useCreateQuotation()

  const [lineItems, setLineItems] = useState<LineItemInput[]>([{ ...EMPTY_LINE_ITEM }])
  const [error, setError] = useState<string | null>(null)

  function updateLineItem(index: number, field: keyof LineItemInput, value: string) {
    setLineItems((prev) =>
      prev.map((li, i) => (i === index ? { ...li, [field]: value } : li)),
    )
  }

  function addLineItem() {
    setLineItems((prev) => [...prev, { ...EMPTY_LINE_ITEM }])
  }

  function removeLineItem(index: number) {
    setLineItems((prev) => prev.filter((_, i) => i !== index))
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { line_items: lineItems } })
    if (result.status === 200) {
      setLineItems([{ ...EMPTY_LINE_ITEM }])
      await queryClient.invalidateQueries({ queryKey: getListQuotationsQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <section>
      <h2>Quotations</h2>

      <form className="card-form" onSubmit={handleSubmit}>
        <h3>New quotation</h3>
        {lineItems.map((li, i) => (
          <div key={i} className="inline-form">
            <input
              type="text"
              placeholder="Description"
              value={li.description}
              onChange={(e) => updateLineItem(i, 'description', e.target.value)}
              required
            />
            <input
              type="text"
              placeholder="Quantity"
              value={li.quantity}
              onChange={(e) => updateLineItem(i, 'quantity', e.target.value)}
              required
            />
            <input
              type="text"
              placeholder="Unit"
              value={li.unit}
              onChange={(e) => updateLineItem(i, 'unit', e.target.value)}
              required
            />
            <input
              type="text"
              placeholder="Unit rate"
              value={li.unit_rate}
              onChange={(e) => updateLineItem(i, 'unit_rate', e.target.value)}
              required
            />
            {lineItems.length > 1 && (
              <button type="button" onClick={() => removeLineItem(i)}>
                Remove
              </button>
            )}
          </div>
        ))}
        <div>
          <button type="button" onClick={addLineItem}>
            Add line item
          </button>
        </div>
        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Creating…' : 'Create quotation'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Version</th>
              <th>Status</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((q) => (
              <tr key={q.id}>
                <td>
                  <Link to={`/quotations/${q.id}`}>v{q.version}</Link>
                </td>
                <td>{q.status}</td>
                <td>{new Date(q.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={3}>No quotations yet.</td>
              </tr>
            )}
          </tbody>
        </table>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </section>
  )
}

function DesignAssetRow({ asset }: { asset: DesignAssetModel }) {
  const queryClient = useQueryClient()
  const revisions = useListDesignRevisions(asset.id)
  const submitRevision = useSubmitDesignRevision()

  const [revising, setRevising] = useState(false)
  const [notes, setNotes] = useState('')
  const [revisionError, setRevisionError] = useState<string | null>(null)

  function startRevision() {
    setRevising(true)
    setNotes('')
    setRevisionError(null)
  }

  async function handleSubmitRevision(e: FormEvent) {
    e.preventDefault()
    setRevisionError(null)
    const result = await submitRevision.mutateAsync({
      id: asset.id,
      data: { notes: notes || null },
    })
    if (result.status === 200) {
      setRevising(false)
      await queryClient.invalidateQueries({ queryKey: getListDesignRevisionsQueryKey(asset.id) })
    } else {
      setRevisionError(result.data.error)
    }
  }

  return (
    <li>
      <h3>{asset.title}</h3>
      {revisions.isPending && <p>Loading revisions…</p>}
      {revisions.data?.status === 200 && (
        <ul className="workstream-list">
          {revisions.data.data.map((rev) => (
            <li key={rev.id}>
              <span className="workstream-badge">{rev.status}</span>
              {rev.notes && <span> {rev.notes}</span>}
            </li>
          ))}
          {revisions.data.data.length === 0 && <li>No revisions yet.</li>}
        </ul>
      )}
      {revising ? (
        <form className="inline-form" onSubmit={handleSubmitRevision}>
          <input
            type="text"
            placeholder="Notes (optional)"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
          />
          <button type="submit" disabled={submitRevision.isPending}>
            {submitRevision.isPending ? 'Submitting…' : 'Submit revision'}
          </button>
          <button type="button" onClick={() => setRevising(false)}>
            Cancel
          </button>
          {revisionError && <p className="form-error">{revisionError}</p>}
        </form>
      ) : (
        <button type="button" onClick={startRevision}>
          New revision
        </button>
      )}
    </li>
  )
}

function DesignSection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListDesignAssets(projectId)
  const create = useCreateDesignAsset()

  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)

  async function handleCreate(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { title } })
    if (result.status === 200) {
      setTitle('')
      await queryClient.invalidateQueries({ queryKey: getListDesignAssetsQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <section>
      <h2>Design</h2>

      <form className="card-form" onSubmit={handleCreate}>
        <h3>New design asset</h3>
        <label>
          Title
          <input type="text" value={title} onChange={(e) => setTitle(e.target.value)} required />
        </label>
        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Adding…' : 'Add design asset'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="design-asset-list">
          {list.data.data.map((asset) => (
            <DesignAssetRow key={asset.id} asset={asset} />
          ))}
          {list.data.data.length === 0 && <li>No design assets yet.</li>}
        </ul>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </section>
  )
}

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

      <QuotationsSection projectId={p.id} />
      <DesignSection projectId={p.id} />
      <ChangeOrdersSection
        projectId={p.id}
        existingWorkstreams={p.workstreams.map((w) => w.workstream_type)}
      />
      <PurchaseOrdersSection projectId={p.id} />
      <ProductionTasksSection projectId={p.id} />
      <SiteExecutionSection projectId={p.id} />
      <ScheduleSection projectId={p.id} />
      <BillingSection projectId={p.id} billingMethod={p.billing_method} />
    </div>
  )
}
