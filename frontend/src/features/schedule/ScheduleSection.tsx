import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListScheduleTaskDependenciesQueryKey,
  getListScheduleTasksQueryKey,
  useAddScheduleTaskDependency,
  useCreateScheduleTask,
  useListScheduleTaskDependencies,
  useListScheduleTasks,
  useUpdateScheduleTaskDates,
  useUpdateScheduleTaskStatus,
} from '../../api/generated/schedule/schedule'
import { WorkstreamType, type WorkstreamType as WorkstreamTypeValue } from '../../api/generated/api.schemas'

const TASK_STATUSES = ['not_started', 'in_progress', 'done'] as const

function ScheduleTaskRow({
  id,
  title,
  status,
  workstreamType,
  plannedStartDate,
  plannedEndDate,
  actualStartDate,
  actualEndDate,
  allTasks,
  projectId,
}: {
  id: string
  title: string
  status: string
  workstreamType: string
  plannedStartDate: string | null | undefined
  plannedEndDate: string | null | undefined
  actualStartDate: string | null | undefined
  actualEndDate: string | null | undefined
  allTasks: { id: string; title: string }[]
  projectId: string
}) {
  const queryClient = useQueryClient()
  const updateStatus = useUpdateScheduleTaskStatus()
  const updateDates = useUpdateScheduleTaskDates()
  const addDependency = useAddScheduleTaskDependency()
  const dependencies = useListScheduleTaskDependencies(id)

  const [plannedStart, setPlannedStart] = useState(plannedStartDate ?? '')
  const [plannedEnd, setPlannedEnd] = useState(plannedEndDate ?? '')
  const [actualStart, setActualStart] = useState(actualStartDate ?? '')
  const [actualEnd, setActualEnd] = useState(actualEndDate ?? '')
  const [dependsOn, setDependsOn] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [shiftNotice, setShiftNotice] = useState<string | null>(null)

  async function handleStatusChange(newStatus: string) {
    setError(null)
    const result = await updateStatus.mutateAsync({ id, data: { status: newStatus } })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListScheduleTasksQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  async function handleDatesSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    setShiftNotice(null)
    const result = await updateDates.mutateAsync({
      id,
      data: {
        planned_start_date: plannedStart || null,
        planned_end_date: plannedEnd || null,
        actual_start_date: actualStart || null,
        actual_end_date: actualEnd || null,
      },
    })
    if (result.status === 200) {
      if (result.data.shifted_dependent_task_ids.length > 0) {
        setShiftNotice(
          `${result.data.shifted_dependent_task_ids.length} dependent task(s) shifted forward.`,
        )
      }
      await queryClient.invalidateQueries({ queryKey: getListScheduleTasksQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  async function handleAddDependency() {
    if (!dependsOn) return
    setError(null)
    const result = await addDependency.mutateAsync({ id, data: { depends_on_task_id: dependsOn } })
    if (result.status === 200) {
      setDependsOn('')
      await queryClient.invalidateQueries({ queryKey: getListScheduleTaskDependenciesQueryKey(id) })
    } else {
      setError(result.data.error)
    }
  }

  const dependencyIds =
    dependencies.data?.status === 200 ? dependencies.data.data.map((d) => d.depends_on_task_id) : []
  const otherTasks = allTasks.filter((t) => t.id !== id && !dependencyIds.includes(t.id))

  return (
    <tr>
      <td>{title}</td>
      <td>
        <span className="workstream-badge">{workstreamType}</span>
      </td>
      <td>
        <select value={status} onChange={(e) => handleStatusChange(e.target.value)} disabled={updateStatus.isPending}>
          {TASK_STATUSES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </td>
      <td>
        <form className="inline-form" onSubmit={handleDatesSubmit}>
          <label>
            Plan start
            <input type="date" value={plannedStart} onChange={(e) => setPlannedStart(e.target.value)} />
          </label>
          <label>
            Plan end
            <input type="date" value={plannedEnd} onChange={(e) => setPlannedEnd(e.target.value)} />
          </label>
          <label>
            Actual start
            <input type="date" value={actualStart} onChange={(e) => setActualStart(e.target.value)} />
          </label>
          <label>
            Actual end
            <input type="date" value={actualEnd} onChange={(e) => setActualEnd(e.target.value)} />
          </label>
          <button type="submit" disabled={updateDates.isPending}>
            Save dates
          </button>
        </form>
        {shiftNotice && <p>{shiftNotice}</p>}
      </td>
      <td>
        {dependencies.data?.status === 200 && (
          <ul className="workstream-list">
            {dependencies.data.data.map((d) => {
              const dep = allTasks.find((t) => t.id === d.depends_on_task_id)
              return <li key={d.depends_on_task_id}>{dep?.title ?? d.depends_on_task_id}</li>
            })}
          </ul>
        )}
        {otherTasks.length > 0 && (
          <span className="inline-form">
            <select value={dependsOn} onChange={(e) => setDependsOn(e.target.value)}>
              <option value="">Depends on…</option>
              {otherTasks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.title}
                </option>
              ))}
            </select>
            <button type="button" onClick={handleAddDependency} disabled={addDependency.isPending}>
              Add
            </button>
          </span>
        )}
        {error && <p className="form-error">{error}</p>}
      </td>
    </tr>
  )
}

export function ScheduleSection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListScheduleTasks(projectId)
  const create = useCreateScheduleTask()

  const [title, setTitle] = useState('')
  const [workstreamType, setWorkstreamType] = useState<WorkstreamTypeValue>(WorkstreamType.design)
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { title, workstream_type: workstreamType } })
    if (result.status === 200) {
      setTitle('')
      await queryClient.invalidateQueries({ queryKey: getListScheduleTasksQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  const tasks = list.data?.status === 200 ? list.data.data : []

  return (
    <section>
      <h2>Schedule</h2>

      <form className="inline-form" onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Task title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          required
        />
        <select value={workstreamType} onChange={(e) => setWorkstreamType(e.target.value as WorkstreamTypeValue)}>
          {Object.values(WorkstreamType).map((wt) => (
            <option key={wt} value={wt}>
              {wt}
            </option>
          ))}
        </select>
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
              <th>Workstream</th>
              <th>Status</th>
              <th>Dates</th>
              <th>Dependencies</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map((t) => (
              <ScheduleTaskRow
                key={t.id}
                id={t.id}
                title={t.title}
                status={t.status}
                workstreamType={t.workstream_type}
                plannedStartDate={t.planned_start_date}
                plannedEndDate={t.planned_end_date}
                actualStartDate={t.actual_start_date}
                actualEndDate={t.actual_end_date}
                allTasks={tasks}
                projectId={projectId}
              />
            ))}
            {tasks.length === 0 && (
              <tr>
                <td colSpan={5}>No schedule tasks yet.</td>
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
