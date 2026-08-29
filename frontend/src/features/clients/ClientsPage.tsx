import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListClientsQueryKey,
  useCreateClient,
  useListClients,
} from '../../api/generated/clients/clients'

export function ClientsPage() {
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)
  const queryClient = useQueryClient()
  const list = useListClients()
  const create = useCreateClient()

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ data: { name } })
    if (result.status === 200) {
      setName('')
      await queryClient.invalidateQueries({ queryKey: getListClientsQueryKey() })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div className="page">
      <h1>Clients</h1>

      <form className="inline-form" onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Client name"
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
            {list.data.data.map((c) => (
              <tr key={c.id}>
                <td>{c.name}</td>
                <td>{new Date(c.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={2}>No clients yet.</td>
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
