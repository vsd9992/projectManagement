import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useListBusinessUnits } from '../../api/generated/business-units/business-units'
import { useListClients } from '../../api/generated/clients/clients'
import {
  getListProjectsQueryKey,
  useCreateProject,
  useListProjects,
} from '../../api/generated/projects/projects'
import { WorkstreamType, type WorkstreamType as WorkstreamTypeValue } from '../../api/generated/api.schemas'

const WORKSTREAM_OPTIONS = Object.values(WorkstreamType)

export function ProjectsListPage() {
  const queryClient = useQueryClient()
  const list = useListProjects()
  const businessUnits = useListBusinessUnits()
  const clients = useListClients()
  const create = useCreateProject()

  const [name, setName] = useState('')
  const [businessUnitId, setBusinessUnitId] = useState('')
  const [clientId, setClientId] = useState('')
  const [workstreams, setWorkstreams] = useState<WorkstreamTypeValue[]>([])
  const [billingMethod, setBillingMethod] = useState<'milestone' | 'progressive'>('milestone')
  const [error, setError] = useState<string | null>(null)

  function toggleWorkstream(wt: WorkstreamTypeValue) {
    setWorkstreams((prev) =>
      prev.includes(wt) ? prev.filter((w) => w !== wt) : [...prev, wt],
    )
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    if (workstreams.length === 0) {
      setError('at least one workstream must be enabled')
      return
    }
    const result = await create.mutateAsync({
      data: {
        name,
        business_unit_id: businessUnitId,
        client_id: clientId,
        workstreams,
        billing_method: billingMethod,
      },
    })
    if (result.status === 200) {
      setName('')
      setWorkstreams([])
      await queryClient.invalidateQueries({ queryKey: getListProjectsQueryKey() })
    } else {
      setError(result.data.error)
    }
  }

  const businessUnitOptions = businessUnits.data?.status === 200 ? businessUnits.data.data : []
  const clientOptions = clients.data?.status === 200 ? clients.data.data : []

  return (
    <div className="page">
      <h1>Projects</h1>

      <form className="card-form" onSubmit={handleSubmit}>
        <h2>New project</h2>
        <label>
          Name
          <input type="text" value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <label>
          Business unit
          <select
            value={businessUnitId}
            onChange={(e) => setBusinessUnitId(e.target.value)}
            required
          >
            <option value="" disabled>
              Select a business unit
            </option>
            {businessUnitOptions.map((bu) => (
              <option key={bu.id} value={bu.id}>
                {bu.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Client
          <select value={clientId} onChange={(e) => setClientId(e.target.value)} required>
            <option value="" disabled>
              Select a client
            </option>
            {clientOptions.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
        <fieldset>
          <legend>Workstreams</legend>
          {WORKSTREAM_OPTIONS.map((wt) => (
            <label key={wt} className="checkbox-label">
              <input
                type="checkbox"
                checked={workstreams.includes(wt)}
                onChange={() => toggleWorkstream(wt)}
              />
              {wt}
            </label>
          ))}
        </fieldset>
        <fieldset>
          <legend>Billing method</legend>
          <label className="checkbox-label">
            <input
              type="radio"
              name="billing_method"
              checked={billingMethod === 'milestone'}
              onChange={() => setBillingMethod('milestone')}
            />
            Milestone
          </label>
          <label className="checkbox-label">
            <input
              type="radio"
              name="billing_method"
              checked={billingMethod === 'progressive'}
              onChange={() => setBillingMethod('progressive')}
            />
            Progressive (RA-bill)
          </label>
        </fieldset>
        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Creating…' : 'Create project'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Billing method</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((p) => (
              <tr key={p.id}>
                <td>
                  <Link to={`/projects/${p.id}`}>{p.name}</Link>
                </td>
                <td>{p.billing_method}</td>
                <td>{new Date(p.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={3}>No projects yet.</td>
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
