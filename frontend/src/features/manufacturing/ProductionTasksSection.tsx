import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListProductionTasksQueryKey,
  useCreateProductionTask,
  useListProductionTasks,
  useUpdateProductionTaskStatus,
} from '../../api/generated/manufacturing/manufacturing'

const STATUSES = ['not_started', 'in_progress', 'completed'] as const

function ProductionTaskRow({
  id,
  title,
  status,
  projectId,
}: {
  id: string
  title: string
  status: string
  projectId: string
}) {
  const queryClient = useQueryClient()
  const updateStatus = useUpdateProductionTaskStatus()
  const [error, setError] = useState<string | null>(null)

  async function handleChange(newStatus: string) {
    setError(null)
    const result = await updateStatus.mutateAsync({ id, data: { status: newStatus } })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListProductionTasksQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <tr>
      <td>{title}</td>
      <td>
        <select value={status} onChange={(e) => handleChange(e.target.value)} disabled={updateStatus.isPending}>
          {STATUSES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        {error && <span className="form-error"> {error}</span>}
      </td>
    </tr>
  )
}

export function ProductionTasksSection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListProductionTasks(projectId)
  const create = useCreateProductionTask()

  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { title } })
    if (result.status === 200) {
      setTitle('')
      await queryClient.invalidateQueries({ queryKey: getListProductionTasksQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <section>
      <h2>Production Tasks</h2>

      <form className="inline-form" onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Task title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          required
        />
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Adding…' : 'Add task'}
        </button>
      </form>
      {error && <p className="form-error">{error}</p>}

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <table>
          <thead>
            <tr>
              <th>Title</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {list.data.data.map((t) => (
              <ProductionTaskRow key={t.id} id={t.id} title={t.title} status={t.status} projectId={projectId} />
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={2}>No production tasks yet.</td>
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
