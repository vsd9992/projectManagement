import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useGetQuotation, useListQuotations } from '../../api/generated/quotations/quotations'
import {
  getListChangeOrdersQueryKey,
  useCreateChangeOrder,
  useListChangeOrders,
} from '../../api/generated/change-orders/change-orders'
import { WorkstreamType, type WorkstreamType as WorkstreamTypeValue } from '../../api/generated/api.schemas'

interface DraftLine {
  originalLineItemId: string | null
  removed: boolean
  description: string
  quantity: string
  unit: string
  unit_rate: string
}

const EMPTY_NEW_LINE: DraftLine = {
  originalLineItemId: null,
  removed: false,
  description: '',
  quantity: '',
  unit: '',
  unit_rate: '',
}

export function ChangeOrdersSection({
  projectId,
  existingWorkstreams,
}: {
  projectId: string
  existingWorkstreams: WorkstreamTypeValue[]
}) {
  const queryClient = useQueryClient()
  const list = useListChangeOrders(projectId)
  const quotations = useListQuotations(projectId)
  const create = useCreateChangeOrder()

  const [baseQuotationId, setBaseQuotationId] = useState('')
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [lines, setLines] = useState<DraftLine[]>([])
  const [addWorkstreams, setAddWorkstreams] = useState<WorkstreamTypeValue[]>([])
  const [error, setError] = useState<string | null>(null)

  const baseQuotation = useGetQuotation(baseQuotationId, {
    query: { enabled: baseQuotationId !== '' },
  })
  const baseLineItems =
    baseQuotation.data?.status === 200 ? baseQuotation.data.data.line_items : []

  const approvedQuotations =
    quotations.data?.status === 200
      ? quotations.data.data.filter((q) => q.status === 'approved')
      : []

  const availableWorkstreams = Object.values(WorkstreamType).filter(
    (wt) => !existingWorkstreams.includes(wt),
  )

  function addNewLine() {
    setLines((prev) => [...prev, { ...EMPTY_NEW_LINE }])
  }

  function addModifyLine(origId: string) {
    const orig = baseLineItems.find((li) => li.id === origId)
    if (!orig) return
    setLines((prev) => [
      ...prev,
      {
        originalLineItemId: orig.id,
        removed: false,
        description: orig.description,
        quantity: orig.quantity,
        unit: orig.unit,
        unit_rate: orig.unit_rate,
      },
    ])
  }

  function updateLine(index: number, field: keyof DraftLine, value: string | boolean) {
    setLines((prev) => prev.map((l, i) => (i === index ? { ...l, [field]: value } : l)))
  }

  function removeLine(index: number) {
    setLines((prev) => prev.filter((_, i) => i !== index))
  }

  function toggleWorkstream(wt: WorkstreamTypeValue) {
    setAddWorkstreams((prev) =>
      prev.includes(wt) ? prev.filter((w) => w !== wt) : [...prev, wt],
    )
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({
      projectId,
      data: {
        base_quotation_id: baseQuotationId,
        title,
        description: description || null,
        line_items: lines.map((l) => ({
          original_line_item_id: l.originalLineItemId,
          removed: l.removed,
          description: l.description,
          quantity: l.quantity,
          unit: l.unit,
          unit_rate: l.unit_rate,
        })),
        add_workstreams: addWorkstreams,
      },
    })
    if (result.status === 200) {
      setTitle('')
      setDescription('')
      setLines([])
      setAddWorkstreams([])
      await queryClient.invalidateQueries({ queryKey: getListChangeOrdersQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <section>
      <h2>Change Orders</h2>

      <form className="card-form" onSubmit={handleSubmit}>
        <h3>New change order</h3>
        <label>
          Base quotation
          <select
            value={baseQuotationId}
            onChange={(e) => {
              setBaseQuotationId(e.target.value)
              setLines([])
            }}
            required
          >
            <option value="" disabled>
              Select an approved quotation
            </option>
            {approvedQuotations.map((q) => (
              <option key={q.id} value={q.id}>
                v{q.version}
              </option>
            ))}
          </select>
        </label>
        <label>
          Title
          <input type="text" value={title} onChange={(e) => setTitle(e.target.value)} required />
        </label>
        <label>
          Description
          <input type="text" value={description} onChange={(e) => setDescription(e.target.value)} />
        </label>

        {baseQuotationId && (
          <fieldset>
            <legend>Line item changes</legend>
            {lines.map((l, i) => (
              <div key={i} className="inline-form">
                {l.originalLineItemId && (
                  <label className="checkbox-label">
                    <input
                      type="checkbox"
                      checked={l.removed}
                      onChange={(e) => updateLine(i, 'removed', e.target.checked)}
                    />
                    Remove
                  </label>
                )}
                <input
                  type="text"
                  placeholder="Description"
                  value={l.description}
                  disabled={l.removed}
                  onChange={(e) => updateLine(i, 'description', e.target.value)}
                  required
                />
                <input
                  type="text"
                  placeholder="Quantity"
                  value={l.quantity}
                  disabled={l.removed}
                  onChange={(e) => updateLine(i, 'quantity', e.target.value)}
                  required
                />
                <input
                  type="text"
                  placeholder="Unit"
                  value={l.unit}
                  disabled={l.removed}
                  onChange={(e) => updateLine(i, 'unit', e.target.value)}
                  required
                />
                <input
                  type="text"
                  placeholder="Unit rate"
                  value={l.unit_rate}
                  disabled={l.removed}
                  onChange={(e) => updateLine(i, 'unit_rate', e.target.value)}
                  required
                />
                <button type="button" onClick={() => removeLine(i)}>
                  Discard
                </button>
              </div>
            ))}
            <div>
              <button type="button" onClick={addNewLine}>
                Add new line
              </button>{' '}
              {baseLineItems.length > 0 && (
                <select
                  value=""
                  onChange={(e) => {
                    if (e.target.value) addModifyLine(e.target.value)
                  }}
                >
                  <option value="">Modify/remove existing line…</option>
                  {baseLineItems.map((li) => (
                    <option key={li.id} value={li.id}>
                      {li.description}
                    </option>
                  ))}
                </select>
              )}
            </div>
          </fieldset>
        )}

        {availableWorkstreams.length > 0 && (
          <fieldset>
            <legend>Add workstreams</legend>
            {availableWorkstreams.map((wt) => (
              <label key={wt} className="checkbox-label">
                <input
                  type="checkbox"
                  checked={addWorkstreams.includes(wt)}
                  onChange={() => toggleWorkstream(wt)}
                />
                {wt}
              </label>
            ))}
          </fieldset>
        )}

        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Proposing…' : 'Propose change order'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Title</th>
              <th>Cost impact</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((co) => (
              <tr key={co.id}>
                <td>
                  <Link to={`/change-orders/${co.id}`}>{co.title}</Link>
                </td>
                <td>{co.cost_impact}</td>
                <td>{co.status}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={3}>No change orders yet.</td>
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
