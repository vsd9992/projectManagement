import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useListVendors } from '../../api/generated/procurement/procurement'
import {
  getListPurchaseOrdersQueryKey,
  useApprovePurchaseOrder,
  useCreatePurchaseOrder,
  useListPurchaseOrders,
  useMarkPurchaseOrderDelivered,
  useRejectPurchaseOrder,
} from '../../api/generated/procurement/procurement'
import type { PoLineItemInput } from '../../api/generated/api.schemas'

const EMPTY_LINE_ITEM: PoLineItemInput = { description: '', quantity: '', unit: '', unit_rate: '' }

function PurchaseOrderRow({ id, title, status }: { id: string; title: string; status: string }) {
  const queryClient = useQueryClient()
  const approve = useApprovePurchaseOrder()
  const reject = useRejectPurchaseOrder()
  const deliver = useMarkPurchaseOrderDelivered()
  const [error, setError] = useState<string | null>(null)

  async function invalidate(projectId: string) {
    await queryClient.invalidateQueries({ queryKey: getListPurchaseOrdersQueryKey(projectId) })
  }

  return (
    <tr>
      <td>{title}</td>
      <td>{status}</td>
      <td>
        {status === 'pending_approval' && (
          <>
            <button
              type="button"
              disabled={approve.isPending}
              onClick={async () => {
                setError(null)
                const result = await approve.mutateAsync({ id, data: {} })
                if (result.status === 200) {
                  await invalidate(result.data.project_id)
                } else {
                  setError(result.data.error)
                }
              }}
            >
              Approve
            </button>{' '}
            <button
              type="button"
              disabled={reject.isPending}
              onClick={async () => {
                setError(null)
                const result = await reject.mutateAsync({ id, data: {} })
                if (result.status === 200) {
                  await invalidate(result.data.project_id)
                } else {
                  setError(result.data.error)
                }
              }}
            >
              Reject
            </button>
          </>
        )}
        {status === 'open' && (
          <button
            type="button"
            disabled={deliver.isPending}
            onClick={async () => {
              setError(null)
              const result = await deliver.mutateAsync({ id })
              if (result.status === 200) {
                await invalidate(result.data.project_id)
              } else {
                setError(result.data.error)
              }
            }}
          >
            Mark delivered
          </button>
        )}
        {error && <span className="form-error"> {error}</span>}
      </td>
    </tr>
  )
}

export function PurchaseOrdersSection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListPurchaseOrders(projectId)
  const vendors = useListVendors()
  const create = useCreatePurchaseOrder()

  const [vendorId, setVendorId] = useState('')
  const [title, setTitle] = useState('')
  const [lineItems, setLineItems] = useState<PoLineItemInput[]>([{ ...EMPTY_LINE_ITEM }])
  const [error, setError] = useState<string | null>(null)

  function updateLineItem(index: number, field: keyof PoLineItemInput, value: string) {
    setLineItems((prev) => prev.map((li, i) => (i === index ? { ...li, [field]: value } : li)))
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
    const result = await create.mutateAsync({
      projectId,
      data: { vendor_id: vendorId, title, line_items: lineItems },
    })
    if (result.status === 200) {
      setTitle('')
      setLineItems([{ ...EMPTY_LINE_ITEM }])
      await queryClient.invalidateQueries({ queryKey: getListPurchaseOrdersQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  const vendorOptions = vendors.data?.status === 200 ? vendors.data.data : []

  return (
    <section>
      <h2>Purchase Orders</h2>

      <form className="card-form" onSubmit={handleSubmit}>
        <h3>New purchase order</h3>
        <label>
          Vendor
          <select value={vendorId} onChange={(e) => setVendorId(e.target.value)} required>
            <option value="" disabled>
              Select a vendor
            </option>
            {vendorOptions.map((v) => (
              <option key={v.id} value={v.id}>
                {v.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Title
          <input type="text" value={title} onChange={(e) => setTitle(e.target.value)} required />
        </label>
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
          {create.isPending ? 'Creating…' : 'Create purchase order'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Title</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((po) => (
              <PurchaseOrderRow key={po.id} id={po.id} title={po.title} status={po.status} />
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={3}>No purchase orders yet.</td>
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
