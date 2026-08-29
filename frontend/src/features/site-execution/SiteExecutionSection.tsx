import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListDailyLogsQueryKey,
  getListPunchListItemsQueryKey,
  getListSiteQueriesQueryKey,
  getListSiteTasksQueryKey,
  useAnswerSiteQuery,
  useClosePunchListItem,
  useCreateDailyLog,
  useCreatePunchListItem,
  useCreateSiteQuery,
  useCreateSiteTask,
  useListDailyLogs,
  useListPunchListItems,
  useListSiteQueries,
  useListSiteTasks,
  useUpdateSiteTaskStatus,
} from '../../api/generated/site-execution/site-execution'

const TASK_STATUSES = ['not_started', 'in_progress', 'done'] as const

function SiteTaskRow({
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
  const updateStatus = useUpdateSiteTaskStatus()
  const [error, setError] = useState<string | null>(null)

  async function handleChange(newStatus: string) {
    setError(null)
    const result = await updateStatus.mutateAsync({ id, data: { status: newStatus } })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListSiteTasksQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <tr>
      <td>{title}</td>
      <td>
        <select value={status} onChange={(e) => handleChange(e.target.value)} disabled={updateStatus.isPending}>
          {TASK_STATUSES.map((s) => (
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

function SiteTasksSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListSiteTasks(projectId)
  const create = useCreateSiteTask()
  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { title } })
    if (result.status === 200) {
      setTitle('')
      await queryClient.invalidateQueries({ queryKey: getListSiteTasksQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Site Tasks</h3>
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
              <SiteTaskRow key={t.id} id={t.id} title={t.title} status={t.status} projectId={projectId} />
            ))}
            {list.data.data.length === 0 && (
              <tr>
                <td colSpan={2}>No site tasks yet.</td>
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

function DailyLogsSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListDailyLogs(projectId)
  const create = useCreateDailyLog()
  const [logDate, setLogDate] = useState('')
  const [notes, setNotes] = useState('')
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { log_date: logDate, notes } })
    if (result.status === 200) {
      setLogDate('')
      setNotes('')
      await queryClient.invalidateQueries({ queryKey: getListDailyLogsQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Daily Logs</h3>
      <form className="inline-form" onSubmit={handleSubmit}>
        <input type="date" value={logDate} onChange={(e) => setLogDate(e.target.value)} required />
        <input
          type="text"
          placeholder="Notes"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          required
        />
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Logging…' : 'Add log'}
        </button>
      </form>
      {error && <p className="form-error">{error}</p>}

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="workstream-list">
          {list.data.data.map((log) => (
            <li key={log.id}>
              <span className="workstream-badge">{log.log_date}</span>
              <span> {log.notes}</span>
            </li>
          ))}
          {list.data.data.length === 0 && <li>No daily logs yet.</li>}
        </ul>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </div>
  )
}

function PunchListSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListPunchListItems(projectId)
  const create = useCreatePunchListItem()
  const close = useClosePunchListItem()
  const [description, setDescription] = useState('')
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { description } })
    if (result.status === 200) {
      setDescription('')
      await queryClient.invalidateQueries({ queryKey: getListPunchListItemsQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  async function handleClose(id: string) {
    setError(null)
    const result = await close.mutateAsync({ id })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListPunchListItemsQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Punch List</h3>
      <form className="inline-form" onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          required
        />
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Raising…' : 'Raise item'}
        </button>
      </form>
      {error && <p className="form-error">{error}</p>}

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="workstream-list">
          {list.data.data.map((item) => (
            <li key={item.id}>
              <span className="workstream-badge">{item.status}</span>
              <span> {item.description}</span>
              {item.status !== 'closed' && (
                <button type="button" onClick={() => handleClose(item.id)} disabled={close.isPending}>
                  Close
                </button>
              )}
            </li>
          ))}
          {list.data.data.length === 0 && <li>No punch list items yet.</li>}
        </ul>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </div>
  )
}

function SiteQueriesSubsection({ projectId }: { projectId: string }) {
  const queryClient = useQueryClient()
  const list = useListSiteQueries(projectId)
  const create = useCreateSiteQuery()
  const answer = useAnswerSiteQuery()
  const [subject, setSubject] = useState('')
  const [question, setQuestion] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [answeringId, setAnsweringId] = useState<string | null>(null)
  const [answerText, setAnswerText] = useState('')

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await create.mutateAsync({ projectId, data: { subject, question } })
    if (result.status === 200) {
      setSubject('')
      setQuestion('')
      await queryClient.invalidateQueries({ queryKey: getListSiteQueriesQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  async function handleAnswer(id: string) {
    setError(null)
    const result = await answer.mutateAsync({ id, data: { answer: answerText } })
    if (result.status === 200) {
      setAnsweringId(null)
      setAnswerText('')
      await queryClient.invalidateQueries({ queryKey: getListSiteQueriesQueryKey(projectId) })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div>
      <h3>Site Queries</h3>
      <form className="inline-form" onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Subject"
          value={subject}
          onChange={(e) => setSubject(e.target.value)}
          required
        />
        <input
          type="text"
          placeholder="Question"
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          required
        />
        <button type="submit" disabled={create.isPending}>
          {create.isPending ? 'Raising…' : 'Raise query'}
        </button>
      </form>
      {error && <p className="form-error">{error}</p>}

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="workstream-list">
          {list.data.data.map((q) => (
            <li key={q.id}>
              <span className="workstream-badge">{q.status}</span>
              <span>
                {' '}
                {q.subject}: {q.question}
              </span>
              {q.answer && <p>Answer: {q.answer}</p>}
              {q.status !== 'answered' &&
                (answeringId === q.id ? (
                  <span className="inline-form">
                    <input
                      type="text"
                      placeholder="Answer"
                      value={answerText}
                      onChange={(e) => setAnswerText(e.target.value)}
                    />
                    <button type="button" onClick={() => handleAnswer(q.id)} disabled={answer.isPending}>
                      Submit
                    </button>
                    <button type="button" onClick={() => setAnsweringId(null)}>
                      Cancel
                    </button>
                  </span>
                ) : (
                  <button type="button" onClick={() => setAnsweringId(q.id)}>
                    Answer
                  </button>
                ))}
            </li>
          ))}
          {list.data.data.length === 0 && <li>No site queries yet.</li>}
        </ul>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </div>
  )
}

export function SiteExecutionSection({ projectId }: { projectId: string }) {
  return (
    <section>
      <h2>Site Execution</h2>
      <SiteTasksSubsection projectId={projectId} />
      <DailyLogsSubsection projectId={projectId} />
      <PunchListSubsection projectId={projectId} />
      <SiteQueriesSubsection projectId={projectId} />
    </section>
  )
}
