import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListBusinessUnitsQueryKey,
  useCreateBusinessUnit,
  useListBusinessUnits,
} from '../../api/generated/business-units/business-units'

export function BusinessUnitsPage() {
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)
  const queryClient = useQueryClient()
  const list = useListBusinessUnits()
  const create = useCreateBusinessUnit()

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ data: { name } })
    if (result.status === 200) {
      setName('')
      await queryClient.invalidateQueries({ queryKey: getListBusinessUnitsQueryKey() })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div className="page">
      <h1>Business Units</h1>

      <form className="inline-form" onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Business unit name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          required
        />
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Adding…' : 'Add'}
        </button>
      </form>
      {error && <p className="form-error">{error}</p>}

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((bu) => (
              <tr key={bu.id}>
                <td>{bu.name}</td>
                <td>{new Date(bu.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={2}>No business units yet.</td>
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
