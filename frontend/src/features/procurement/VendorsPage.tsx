import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListVendorsQueryKey,
  useCreateVendor,
  useListVendors,
} from '../../api/generated/procurement/procurement'

export function VendorsPage() {
  const [name, setName] = useState('')
  const [contactEmail, setContactEmail] = useState('')
  const [contactPhone, setContactPhone] = useState('')
  const [error, setError] = useState<string | null>(null)
  const queryClient = useQueryClient()
  const list = useListVendors()
  const create = useCreateVendor()

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({
      data: {
        name,
        contact_email: contactEmail || null,
        contact_phone: contactPhone || null,
      },
    })
    if (result.status === 200) {
      setName('')
      setContactEmail('')
      setContactPhone('')
      await queryClient.invalidateQueries({ queryKey: getListVendorsQueryKey() })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div className="page">
      <h1>Vendors</h1>

      <form className="card-form" onSubmit={handleSubmit}>
        <h2>New vendor</h2>
        <label>
          Name
          <input type="text" value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <label>
          Contact email
          <input
            type="email"
            value={contactEmail}
            onChange={(e) => setContactEmail(e.target.value)}
          />
        </label>
        <label>
          Contact phone
          <input
            type="text"
            value={contactPhone}
            onChange={(e) => setContactPhone(e.target.value)}
          />
        </label>
        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Adding…' : 'Add vendor'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Email</th>
              <th>Phone</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((v) => (
              <tr key={v.id}>
                <td>{v.name}</td>
                <td>{v.contact_email ?? '—'}</td>
                <td>{v.contact_phone ?? '—'}</td>
                <td>{new Date(v.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={4}>No vendors yet.</td>
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
