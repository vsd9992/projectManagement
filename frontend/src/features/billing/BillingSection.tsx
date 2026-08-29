import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListInvoicesQueryKey,
  getListMilestonesQueryKey,
  useCompleteMilestone,
  useCreateInvoice,
  useCreateMilestone,
  useListInvoices,
  useListMilestones,
  useMarkInvoicePaid,
} from '../../api/generated/billing/billing'

function MilestonesSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListMilestones(projectId)
  const create = useCreateMilestone()
  const complete = useCompleteMilestone()
  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { title } })
    if (result.status === 200) {
      setTitle('')
      await queryClient.invalidateQueries({ queryKey: getListMilestonesQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  async function handleComplete(id: string) {
    setError(null)
    const result = await complete.mutateAsync({ id })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListMilestonesQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Milestones</h3>
      <form className="inline-form" onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Milestone title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          required
        />
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Adding…' : 'Add milestone'}
        </button>
      </form>
      {error && <p className="form-error">{error}</p>}

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="workstream-list">
          {list.data.data.map((m) => (
            <li key={m.id}>
              <span className="workstream-badge">{m.status}</span>
              <span> {m.title}</span>
              {m.status !== 'completed' && (
                <button type="button" onClick={() => handleComplete(m.id)} disabled={complete.isPending}>
                  Mark completed
                </button>
              )}
            </li>
          ))}
          {list.data.data.length === 0 && <li>No milestones yet.</li>}
        </ul>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </div>
  )
}

function InvoicesSubsection({
  projectId,
  billingMethod,
}: {
  projectId: string
  billingMethod: string
}) {
  const queryClient = useQueryClient()
  const list = useListInvoices(projectId)
  const milestones = useListMilestones(projectId)
  const create = useCreateInvoice()
  const markPaid = useMarkInvoicePaid()

  const [milestoneId, setMilestoneId] = useState('')
  const [baseAmount, setBaseAmount] = useState('')
  const [certifiedValue, setCertifiedValue] = useState('')
  const [retentionPercent, setRetentionPercent] = useState('0')
  const [error, setError] = useState<string | null>(null)

  const completedMilestones =
    milestones.data?.status === 200
      ? milestones.data.data.filter((m) => m.status === 'completed')
      : []

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({
      projectId,
      data:
        billingMethod === 'progressive'
          ? {
              billing_method: 'progressive',
              certified_value_to_date: certifiedValue,
              retention_percent: retentionPercent,
            }
          : {
              billing_method: 'milestone',
              milestone_id: milestoneId,
              base_amount: baseAmount,
              retention_percent: retentionPercent,
            },
    })
    if (result.status === 200) {
      setMilestoneId('')
      setBaseAmount('')
      setCertifiedValue('')
      await queryClient.invalidateQueries({ queryKey: getListInvoicesQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  async function handleMarkPaid(id: string) {
    setError(null)
    const result = await markPaid.mutateAsync({ id })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListInvoicesQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Invoices</h3>
      <form className="card-form" onSubmit={handleSubmit}>
        <h4>Raise invoice ({billingMethod})</h4>
        {billingMethod === 'progressive' ? (
          <label>
            Certified value to date (cumulative)
            <input
              type="text"
              value={certifiedValue}
              onChange={(e) => setCertifiedValue(e.target.value)}
              required
            />
          </label>
        ) : (
          <>
            <label>
              Milestone
              <select value={milestoneId} onChange={(e) => setMilestoneId(e.target.value)} required>
                <option value="" disabled>
                  Select a completed milestone
                </option>
                {completedMilestones.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.title}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Base amount
              <input
                type="text"
                value={baseAmount}
                onChange={(e) => setBaseAmount(e.target.value)}
                required
              />
            </label>
          </>
        )}
        <label>
          Retention %
          <input
            type="text"
            value={retentionPercent}
            onChange={(e) => setRetentionPercent(e.target.value)}
            required
          />
        </label>
        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Raising…' : 'Raise invoice'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Base amount</th>
              <th>GST</th>
              <th>GST TDS</th>
              <th>Retention</th>
              <th>Net payable</th>
              <th>Status</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((inv) => (
              <tr key={inv.id}>
                <td>{inv.base_amount}</td>
                <td>{inv.gst_amount}</td>
                <td>{inv.gst_tds_amount}</td>
                <td>{inv.retention_amount}</td>
                <td>{inv.net_payable}</td>
                <td>{inv.status}</td>
                <td>
                  {inv.status !== 'paid' && (
                    <button type="button" onClick={() => handleMarkPaid(inv.id)} disabled={markPaid.isPending}>
                      Mark paid
                    </button>
                  )}
                </td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={7}>No invoices yet.</td>
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

export function BillingSection({
  projectId,
  billingMethod,
}: {
  projectId: string
  billingMethod: string
}) {
  return (
    <section>
      <h2>Billing</h2>
      <MilestonesSubsection projectId={projectId} />
      <InvoicesSubsection projectId={projectId} billingMethod={billingMethod} />
    </section>
  )
}
