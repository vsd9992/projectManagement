import { useQueryClient } from '@tanstack/react-query'
import {
  getListTenantsQueryKey,
  useDeleteTenant,
  useListTenants,
  usePauseTenant,
  useResumeTenant,
} from '../../api/generated/platform/platform'

export function PlatformTenantsPage() {
  const queryClient = useQueryClient()
  const list = useListTenants()
  const pause = usePauseTenant()
  const resume = useResumeTenant()
  const del = useDeleteTenant()

  async function invalidate() {
    await queryClient.invalidateQueries({ queryKey: getListTenantsQueryKey() })
  }

  return (
    <div className="page">
      <h1>Tenants</h1>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((t) => (
              <tr key={t.id}>
                <td>{t.name}</td>
                <td>{t.status}</td>
                <td>
                  {t.status === 'active' && (
                    <button
                      type="button"
                      disabled={pause.isPending}
                      onClick={async () => {
                        await pause.mutateAsync({ id: t.id })
                        await invalidate()
                      }}
                    >
                      Pause
                    </button>
                  )}
                  {t.status === 'paused' && (
                    <button
                      type="button"
                      disabled={resume.isPending}
                      onClick={async () => {
                        await resume.mutateAsync({ id: t.id })
                        await invalidate()
                      }}
                    >
                      Resume
                    </button>
                  )}
                  {t.status !== 'deleted' && (
                    <button
                      type="button"
                      disabled={del.isPending}
                      onClick={async () => {
                        if (!window.confirm(`Delete tenant "${t.name}"? This cannot be undone.`)) {
                          return
                        }
                        await del.mutateAsync({ id: t.id })
                        await invalidate()
                      }}
                    >
                      Delete
                    </button>
                  )}
                </td>
              </tr>
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={3}>No tenants yet.</td>
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
