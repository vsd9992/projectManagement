import { useQueryClient } from '@tanstack/react-query'
import {
  getListMyNotificationsQueryKey,
  useListMyNotifications,
  useMarkNotificationRead,
} from '../../api/generated/notifications/notifications'

export function NotificationsPage() {
  const queryClient = useQueryClient()
  const list = useListMyNotifications()
  const markRead = useMarkNotificationRead()

  async function handleMarkRead(id: string) {
    const result = await markRead.mutateAsync({ id })
    if (result.status === 200) {
      await queryClient.invalidateQueries({ queryKey: getListMyNotificationsQueryKey() })
    }
  }

  return (
    <div className="page">
      <h1>Notifications</h1>

      {list.isPending && <p>Loading…</p>}
      {list.data?.status === 200 && (
        <ul className="workstream-list">
          {list.data.data.map((n) => (
            <li key={n.id}>
              <span className="workstream-badge">{n.is_read ? 'read' : 'unread'}</span>
              <span> {n.message}</span>
              <span> ({new Date(n.created_at).toLocaleString()})</span>
              {!n.is_read && (
                <button type="button" onClick={() => handleMarkRead(n.id)} disabled={markRead.isPending}>
                  Mark read
                </button>
              )}
            </li>
          ))}
          {list.data.data.length === 0 && <li>No notifications.</li>}
        </ul>
      )}
      {list.data && list.data.status !== 200 && (
        <p className="form-error">{list.data.data.error}</p>
      )}
    </div>
  )
}
