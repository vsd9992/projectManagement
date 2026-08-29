import { Fragment, useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { useListBusinessUnits } from '../../api/generated/business-units/business-units'
import { useListClients } from '../../api/generated/clients/clients'
import {
  getListLeadsQueryKey,
  useConvertLead,
  useCreateLead,
  useListLeads,
} from '../../api/generated/leads/leads'
import { WorkstreamType, type WorkstreamType as WorkstreamTypeValue } from '../../api/generated/api.schemas'

const WORKSTREAM_OPTIONS = Object.values(WorkstreamType)

export function LeadsPage() {
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  const businessUnits = useListBusinessUnits()
  const clients = useListClients()
  const list = useListLeads()
  const create = useCreateLead()
  const convert = useConvertLead()

  const [businessUnitId, setBusinessUnitId] = useState('')
  const [clientId, setClientId] = useState('')
  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)

  const [convertingLeadId, setConvertingLeadId] = useState<string | null>(null)
  const [projectName, setProjectName] = useState('')
  const [workstreams, setWorkstreams] = useState<WorkstreamTypeValue[]>([])
  const [billingMethod, setBillingMethod] = useState<'milestone' | 'progressive'>('milestone')
  const [convertError, setConvertError] = useState<string | null>(null)

  const businessUnitOptions = businessUnits.data?.status === 200 ? businessUnits.data.data : []
  const clientOptions = clients.data?.status === 200 ? clients.data.data : []

  function businessUnitName(id: string) {
    return businessUnitOptions.find((bu) => bu.id === id)?.name ?? id
  }
  function clientName(id: string) {
    return clientOptions.find((c) => c.id === id)?.name ?? id
  }

  async function handleCreate(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({
      data: { business_unit_id: businessUnitId, client_id: clientId, title },
    })
    if (result.status === 200) {
      setTitle('')
      await queryClient.invalidateQueries({ queryKey: getListLeadsQueryKey() })
    } else {
      setError(result.data.error)
    }
  }

  function startConvert(leadId: string) {
    setConvertingLeadId(leadId)
    setProjectName('')
    setWorkstreams([])
    setBillingMethod('milestone')
    setConvertError(null)
  }

  function toggleWorkstream(wt: WorkstreamTypeValue) {
    setWorkstreams((prev) => (prev.includes(wt) ? prev.filter((w) => w !== wt) : [...prev, wt]))
  }

  async function handleConvert(e: FormEvent) {
    e.preventDefault()
    if (!convertingLeadId) return
    setConvertError(null)
    if (workstreams.length === 0) {
      setConvertError('at least one workstream must be enabled')
      return
    }
    const result = await convert.mutateAsync({
      id: convertingLeadId,
      data: { project_name: projectName, workstreams, billing_method: billingMethod },
    })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListLeadsQueryKey() })
      navigate(`/projects/${result.data.id}`)
    } else {
      setConvertError(result.data.error)
    }
  }

  return (
    <div className="page">
      <h1>Leads</h1>

      <form className="card-form" onSubmit={handleCreate}>
        <h2>New lead</h2>
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
        <label>
          Title
          <input type="text" value={title} onChange={(e) => setTitle(e.target.value)} required />
        </label>
        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Adding…' : 'Add lead'}
        </button>
      </form>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Title</th>
              <th>Client</th>
              <th>Business unit</th>
              <th>Status</th>
              <th>Created</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((lead) => (
              <Fragment key={lead.id}>
                <tr>
                  <td>{lead.title}</td>
                  <td>{clientName(lead.client_id)}</td>
                  <td>{businessUnitName(lead.business_unit_id)}</td>
                  <td>{lead.status}</td>
                  <td>{new Date(lead.created_at).toLocaleDateString()}</td>
                  <td>
                    {lead.status === 'new' || lead.status === 'qualified' ? (
                      <button type="button" onClick={() => startConvert(lead.id)}>
                        Convert to project
                      </button>
                    ) : null}
                  </td>
                </tr>
                {convertingLeadId === lead.id && (
                  <tr>
                    <td colSpan={6}>
                      <form className="card-form" onSubmit={handleConvert}>
                        <h3>Convert "{lead.title}" to a project</h3>
                        <label>
                          Project name
                          <input
                            type="text"
                            value={projectName}
                            onChange={(e) => setProjectName(e.target.value)}
                            required
                          />
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
                              name="convert_billing_method"
                              checked={billingMethod === 'milestone'}
                              onChange={() => setBillingMethod('milestone')}
                            />
                            Milestone
                          </label>
                          <label className="checkbox-label">
                            <input
                              type="radio"
                              name="convert_billing_method"
                              checked={billingMethod === 'progressive'}
                              onChange={() => setBillingMethod('progressive')}
                            />
                            Progressive (RA-bill)
                          </label>
                        </fieldset>
                        {convertError && <p className="form-error">{convertError}</p>}
                        <div>
                          <button type="submit" disabled={convert.isPending}>
                            {convert.isPending ? 'Converting…' : 'Convert'}
                          </button>
                          <button type="button" onClick={() => setConvertingLeadId(null)}>
                            Cancel
                          </button>
                        </div>
                      </form>
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={6}>No leads yet.</td>
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
