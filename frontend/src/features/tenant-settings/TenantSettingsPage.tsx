import { useEffect, useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getGetTenantSettingsQueryKey,
  useGetTenantSettings,
  useUpdateTenantSettings,
} from '../../api/generated/tenant-settings/tenant-settings'

export function TenantSettingsPage() {
  const queryClient = useQueryClient()
  const settings = useGetTenantSettings()
  const update = useUpdateTenantSettings()

  const [workstreamLabels, setWorkstreamLabels] = useState('{}')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (settings.data?.status === 200) {
      setWorkstreamLabels(JSON.stringify(settings.data.data.workstream_labels, null, 2))
    }
  }, [settings.data])

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    let parsed: unknown
    try {
      parsed = JSON.parse(workstreamLabels)
    } catch {
      setError('workstream_labels must be valid JSON')
      return
    }
    const result = await update.mutateAsync({ data: { workstream_labels: parsed } })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getGetTenantSettingsQueryKey() })
    } else {
      setError(result.data.error)
    }
  }

  if (settings.isPending) {
    return (
      <div className="page">
        <p>Loading…</p>
      </div>
    )
  }

  if (settings.data?.status !== 200) {
    return (
      <div className="page">
        <p className="form-error">{settings.data?.data.error ?? 'Failed to load settings.'}</p>
      </div>
    )
  }

  return (
    <div className="page">
      <h1>Tenant Settings</h1>
      <dl className="detail-list">
        <dt>Region profile</dt>
        <dd>{settings.data.data.region_profile}</dd>
      </dl>

      <form className="card-form" onSubmit={handleSubmit}>
        <h2>Workstream labels</h2>
        <label>
          JSON overrides (e.g. {'{"design": "Design & Styling"}'})
          <textarea
            value={workstreamLabels}
            onChange={(e) => setWorkstreamLabels(e.target.value)}
            rows={8}
          />
        </label>
        {error && <p className="form-error">{error}</p>}
        <button type="submit" disabled={update.isPending}>
          {update.isPending ? 'Saving…' : 'Save'}
        </button>
      </form>
    </div>
  )
}
